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

// --- Helpers (translated from the C macros) ---

#[inline]
fn is_left(index: u32) -> bool {
    (index & 1) == 1
}

#[inline]
fn parent_of(index: u32) -> u32 {
    if index == 0 {
        0
    } else {
        (index - 1) >> 1
    }
}

#[inline]
fn sibling_of(index: u32) -> u32 {
    if index == 0 {
        0
    } else {
        ((index + 1) ^ 1) - 1
    }
}

// --- Public API ---

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
    for i in 0..n - 1 {
        for j in i + 1..n {
            if cmp(&slice[i], &slice[j]) > 0 {
                slice.swap(i, j);
            }
        }
    }
}

pub fn cbmt_uint32_reverse_cmp(left: &u32, right: &u32) -> i32 {
    // reverse order: right - left (matching C's uint32_t arithmetic + cast to int)
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
    indices.capacity = values.capacity();
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
    item[..queue.width]
        .copy_from_slice(&queue.buffer.data[tail_offset..tail_offset + queue.width]);
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
    // CBMT_NODE_SIZE == 4 in this build, matching the I32 mode in the C source.
    let left_value = i32::from_le_bytes(left.bytes);
    let right_value = i32::from_le_bytes(right.bytes);
    left_value.wrapping_sub(right_value)
}

pub fn cbmt_node_pair_reverse_cmp(left: &CbmtNodePair, right: &CbmtNodePair) -> i32 {
    // reverse order: right.index - left.index
    right.index.wrapping_sub(left.index) as i32
}

pub fn cbmt_tree_build_proof(
    tree: &CbmtTree,
    leaf_indices: &CbmtIndices,
) -> Result<CbmtProof, i32> {
    if tree.length == 0 || leaf_indices.values.is_empty() {
        return Err(CBMT_ERROR_BUILD_PROOF);
    }
    let leaves_count = ((tree.length >> 1) + 1) as u32;

    // Build initial queue, sort it (descending), then process.
    let mut initial: Vec<u32> = leaf_indices
        .values
        .iter()
        .map(|&v| v + (leaves_count - 1))
        .collect();
    cbmt_simple_bubble_sort(&mut initial, cbmt_uint32_reverse_cmp);

    let first_value = match initial.first() {
        Some(&v) => v,
        None => return Err(CBMT_ERROR_BUILD_PROOF),
    };
    if first_value >= (leaves_count << 1) - 1 {
        return Err(CBMT_ERROR_BUILD_PROOF);
    }

    let mut queue: VecDeque<u32> = initial.into_iter().collect();
    let mut lemmas: Vec<CbmtNode> = Vec::new();

    while let Some(index) = queue.pop_front() {
        if index == 0 {
            if !queue.is_empty() {
                return Err(CBMT_FATAL_BUILD_PROOF);
            }
            break;
        }
        let sibling = sibling_of(index);
        let consume_sibling = match queue.front() {
            Some(&front_v) if front_v == sibling => true,
            _ => false,
        };
        if consume_sibling {
            queue.pop_front();
        } else {
            // Add the sibling node from the tree to the lemmas list.
            let sibling_idx = sibling as usize;
            if sibling_idx >= tree.nodes.len() {
                return Err(CBMT_FATAL_BUILD_PROOF);
            }
            lemmas.push(tree.nodes[sibling_idx].clone());
        }
        let parent = parent_of(index);
        if parent != 0 {
            queue.push_back(parent);
        }
    }

    // Build proof.indices: leaf_indices offset by (leaves_count - 1), then sorted by node value.
    let mut values: Vec<u32> = leaf_indices
        .values
        .iter()
        .map(|&v| v + (leaves_count - 1))
        .collect();

    let n = values.len();
    if n > 1 {
        for i in 0..n - 1 {
            for j in i + 1..n {
                let left_index = values[i];
                let right_index = values[j];
                let order = cbmt_node_cmp(
                    &tree.nodes[left_index as usize],
                    &tree.nodes[right_index as usize],
                );
                if order > 0 {
                    values[i] = right_index;
                    values[j] = left_index;
                }
            }
        }
    }

    let cap = values.capacity();
    Ok(CbmtProof {
        indices: CbmtIndices {
            values,
            capacity: cap,
        },
        lemmas,
    })
}

pub fn cbmt_tree_root(tree: &CbmtTree) -> CbmtNode {
    if tree.length == 0 || tree.nodes.is_empty() {
        CbmtNode {
            bytes: [0u8; CBMT_NODE_SIZE],
        }
    } else {
        tree.nodes[0].clone()
    }
}

/// Internal helper: compute the proof root using a generic merge callable.
fn proof_root_compute<F>(
    proof: &CbmtProof,
    leaves: &CbmtLeaves,
    mut merge: F,
) -> Result<CbmtNode, i32>
where
    F: FnMut(&CbmtNode, &CbmtNode) -> CbmtNode,
{
    if leaves.nodes.len() != proof.indices.values.len() || leaves.nodes.is_empty() {
        return Err(CBMT_ERROR_PROOF_ROOT);
    }

    // Clone and sort leaves to align with the proof's (already sorted-by-value) indices.
    let mut leaves_clone: Vec<CbmtNode> = leaves.nodes.clone();
    cbmt_simple_bubble_sort(&mut leaves_clone, cbmt_node_cmp);

    // Build pair queue.
    let mut pairs: Vec<CbmtNodePair> = (0..leaves.nodes.len())
        .map(|i| CbmtNodePair {
            index: proof.indices.values[i],
            node: leaves_clone[i].clone(),
        })
        .collect();
    cbmt_simple_bubble_sort(&mut pairs, cbmt_node_pair_reverse_cmp);

    let mut queue: VecDeque<CbmtNodePair> = pairs.into_iter().collect();
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

        let sib_index = sibling_of(index);
        let sibling: Option<CbmtNode> = match queue.front() {
            Some(p) if p.index == sib_index => {
                let pair_sibling = queue.pop_front().unwrap();
                Some(pair_sibling.node)
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
            let parent = if is_left(index) {
                merge(&node, &sibling)
            } else {
                merge(&sibling, &node)
            };
            queue.push_back(CbmtNodePair {
                index: parent_of(index),
                node: parent,
            });
        }
    }
    // If we exit without ever hitting index 0, this is an error.
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
    match proof_root_compute(proof, leaves, |a, b| merge(merge_ctx, a, b)) {
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
    // The C version expects `leaves->length == proof->indices.length`. The Rust callers
    // (see test_rebuild_proof) pass the original full leaves array. If lengths differ,
    // extract the needed leaves using the proof indices (which are tree-level indices,
    // offset by leaves_count - 1).
    let needed_leaves: CbmtLeaves = if leaves.nodes.len() == proof.indices.values.len() {
        leaves.clone()
    } else {
        let leaves_count = leaves.nodes.len();
        if leaves_count == 0 {
            return CBMT_ERROR_PROOF_ROOT;
        }
        let offset = leaves_count as u32 - 1;
        let mut nodes = Vec::with_capacity(proof.indices.values.len());
        for &tree_idx in &proof.indices.values {
            if tree_idx < offset {
                return CBMT_ERROR_PROOF_ROOT;
            }
            let leaf_idx = (tree_idx - offset) as usize;
            if leaf_idx >= leaves_count {
                return CBMT_ERROR_PROOF_ROOT;
            }
            nodes.push(leaves.nodes[leaf_idx].clone());
        }
        CbmtLeaves { nodes }
    };

    match proof_root_compute(proof, &needed_leaves, |a, b| merge(None, a, b)) {
        Ok(computed) => {
            if computed.bytes == expected_root.bytes {
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
        return Ok(CbmtNode {
            bytes: [0u8; CBMT_NODE_SIZE],
        });
    }

    let mut queue: VecDeque<CbmtNode> = VecDeque::new();
    // for (int i = length - 1; i > 0; i -= 2)
    let mut i: i64 = length as i64 - 1;
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
    if leaves.nodes.is_empty() {
        tree.nodes = Vec::new();
        tree.length = 0;
        tree.capacity = 0;
        return 0;
    }
    let length = leaves.nodes.len() * 2 - 1;
    tree.nodes = vec![CbmtNode::default(); length];
    tree.length = length;
    tree.capacity = length;

    let offset = leaves.nodes.len() - 1;
    for i in 0..leaves.nodes.len() {
        tree.nodes[offset + i] = leaves.nodes[i].clone();
    }
    for i in 0..leaves.nodes.len() - 1 {
        let rev_idx = leaves.nodes.len() - 2 - i;
        let left_idx = (rev_idx << 1) + 1;
        let right_idx = (rev_idx << 1) + 2;
        let left = tree.nodes[left_idx].clone();
        let right = tree.nodes[right_idx].clone();
        tree.nodes[rev_idx] = merge(None, &left, &right);
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
    // Build the tree using the provided generic merge function.
    let mut tree = CbmtTree::default();
    if leaves.nodes.is_empty() {
        tree.length = 0;
    } else {
        let length = leaves.nodes.len() * 2 - 1;
        tree.nodes = vec![CbmtNode::default(); length];
        tree.length = length;
        tree.capacity = length;
        let offset = leaves.nodes.len() - 1;
        for i in 0..leaves.nodes.len() {
            tree.nodes[offset + i] = leaves.nodes[i].clone();
        }
        for i in 0..leaves.nodes.len() - 1 {
            let rev_idx = leaves.nodes.len() - 2 - i;
            let left_idx = (rev_idx << 1) + 1;
            let right_idx = (rev_idx << 1) + 2;
            let left = tree.nodes[left_idx].clone();
            let right = tree.nodes[right_idx].clone();
            tree.nodes[rev_idx] = merge(merge_ctx, &left, &right);
        }
    }

    match cbmt_tree_build_proof(&tree, leaf_indices) {
        Ok(p) => {
            *proof = p;
            0
        }
        Err(e) => e,
    }
}
