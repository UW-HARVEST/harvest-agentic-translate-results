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

// Internal helpers replicating C macros.
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
    let n = width.min(left.len()).min(right.len());
    for i in 0..n {
        std::mem::swap(&mut left[i], &mut right[i]);
    }
}

pub fn cbmt_simple_bubble_sort<T>(slice: &mut [T], cmp: fn(&T, &T) -> i32) {
    let n = slice.len();
    if n < 2 {
        return;
    }
    for i in 0..n - 1 {
        for j in (i + 1)..n {
            if cmp(&slice[i], &slice[j]) > 0 {
                slice.swap(i, j);
            }
        }
    }
}

pub fn cbmt_uint32_reverse_cmp(left: &u32, right: &u32) -> i32 {
    // Mirrors the C version: returns right - left (with wrapping).
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
    for i in 0..CBMT_NODE_SIZE {
        let cmp = (left.bytes[i] as i32) - (right.bytes[i] as i32);
        if cmp != 0 {
            return cmp;
        }
    }
    0
}

pub fn cbmt_node_pair_reverse_cmp(left: &CbmtNodePair, right: &CbmtNodePair) -> i32 {
    right.index.wrapping_sub(left.index) as i32
}

/// Build the proof from the tree, given leaf indices into the original
/// (pre-tree) leaf list. Mirrors `cbmt_tree_build_proof` in C.
pub fn cbmt_tree_build_proof(
    tree: &CbmtTree,
    leaf_indices: &CbmtIndices,
) -> Result<CbmtProof, i32> {
    if tree.length == 0 || leaf_indices.values.is_empty() {
        return Err(CBMT_ERROR_BUILD_PROOF);
    }
    let leaves_count: u32 = ((tree.length >> 1) + 1) as u32;

    // Working queue of tree-indices (uint32_t equivalent), implemented as a VecDeque.
    let mut queue: std::collections::VecDeque<u32> = std::collections::VecDeque::new();
    for &v in &leaf_indices.values {
        let value = v + (leaves_count - 1);
        queue.push_back(value);
    }
    // Sort initial queue contents in reverse order (largest indices first).
    let mut tmp: Vec<u32> = queue.drain(..).collect();
    cbmt_simple_bubble_sort(&mut tmp, cbmt_uint32_reverse_cmp);
    for v in tmp {
        queue.push_back(v);
    }

    let first_value = *queue.front().expect("queue empty after init");
    if first_value >= (leaves_count << 1) - 1 {
        return Err(CBMT_ERROR_BUILD_PROOF);
    }

    let mut lemmas: Vec<CbmtNode> = Vec::new();

    while !queue.is_empty() {
        let index = queue.pop_front().unwrap();
        if index == 0 {
            if !queue.is_empty() {
                return Err(CBMT_FATAL_BUILD_PROOF);
            }
            break;
        }

        let sibling = cbmt_sibling(index);
        let pop_sibling = matches!(queue.front(), Some(&f) if f == sibling);
        if pop_sibling {
            queue.pop_front();
        } else {
            let src_lemma = &tree.nodes[sibling as usize];
            lemmas.push(src_lemma.clone());
        }

        let parent = cbmt_parent(index);
        if parent != 0 {
            queue.push_back(parent);
        }
    }

    // Build sorted indices: leaf_indices offset by (leaves_count - 1), then
    // permuted to match the order obtained by sorting the corresponding tree
    // nodes ascending. Mirrors the bubble-sort in the C source.
    let mut indices_values: Vec<u32> = leaf_indices
        .values
        .iter()
        .map(|v| v + (leaves_count - 1))
        .collect();

    let len = indices_values.len();
    if len > 1 {
        for i in 0..len - 1 {
            for j in i + 1..len {
                let li = indices_values[i] as usize;
                let ri = indices_values[j] as usize;
                if cbmt_node_cmp(&tree.nodes[li], &tree.nodes[ri]) > 0 {
                    indices_values.swap(i, j);
                }
            }
        }
    }

    Ok(CbmtProof {
        indices: CbmtIndices {
            capacity: indices_values.len(),
            values: indices_values,
        },
        lemmas,
    })
}

pub fn cbmt_tree_root(tree: &CbmtTree) -> CbmtNode {
    if tree.length == 0 {
        CbmtNode {
            bytes: [0; CBMT_NODE_SIZE],
        }
    } else {
        tree.nodes[0].clone()
    }
}

/// Generic version of proof root computation. The simpler verification uses
/// `cbmt_proof_verify` below (which does not require external buffers).
pub fn cbmt_proof_root<Ctx>(
    proof: &CbmtProof,
    root: &mut CbmtNode,
    leaves: &CbmtLeaves,
    merge: CbmtNodeMergeFn<Ctx>,
    merge_ctx: &mut Ctx,
    _nodes_buffer: CbmtBuffer,
    _pairs_buffer: CbmtBuffer,
) -> i32 {
    if leaves.nodes.len() != proof.indices.values.len() || leaves.nodes.is_empty() {
        return CBMT_ERROR_PROOF_ROOT;
    }

    // Clone leaves and sort ascending.
    let mut leaves_clone: Vec<CbmtNode> = leaves.nodes.clone();
    cbmt_simple_bubble_sort(&mut leaves_clone, cbmt_node_cmp);

    let mut pairs: Vec<CbmtNodePair> = (0..leaves.nodes.len())
        .map(|i| CbmtNodePair {
            index: proof.indices.values[i],
            node: leaves_clone[i].clone(),
        })
        .collect();

    cbmt_simple_bubble_sort(&mut pairs, cbmt_node_pair_reverse_cmp);

    let mut queue: std::collections::VecDeque<CbmtNodePair> = pairs.into_iter().collect();
    let mut lemmas_offset = 0usize;

    while let Some(pair_current) = queue.pop_front() {
        let index = pair_current.index;
        let node = &pair_current.node;

        if index == 0 {
            if proof.lemmas.len() == lemmas_offset && queue.is_empty() {
                cbmt_node_copy(root, node);
                return 0;
            } else {
                return CBMT_ERROR_PROOF_ROOT;
            }
        }

        let sibling_index = cbmt_sibling(index);
        let mut sibling: Option<CbmtNode> = None;
        if let Some(front) = queue.front() {
            if front.index == sibling_index {
                let popped = queue.pop_front().unwrap();
                sibling = Some(popped.node);
            }
        }
        if sibling.is_none() && lemmas_offset < proof.lemmas.len() {
            sibling = Some(proof.lemmas[lemmas_offset].clone());
            lemmas_offset += 1;
        }

        if let Some(sib) = sibling {
            let parent = if cbmt_is_left(index) {
                merge(merge_ctx, node, &sib)
            } else {
                merge(merge_ctx, &sib, node)
            };
            queue.push_back(CbmtNodePair {
                index: cbmt_parent(index),
                node: parent,
            });
        }
    }
    0
}

/// Verify a Merkle proof against an expected root. The `leaves` argument is
/// the original list of leaves (the same one used to build the tree); the
/// proof's indices identify which of those leaves are being proven.
pub fn cbmt_proof_verify(
    proof: &CbmtProof,
    expected_root: &CbmtNode,
    leaves: &CbmtLeaves,
    merge: fn(Option<&mut ()>, &CbmtNode, &CbmtNode) -> CbmtNode,
) -> i32 {
    let leaves_count = leaves.nodes.len();
    if leaves_count == 0 {
        return CBMT_ERROR_PROOF_ROOT;
    }
    if proof.indices.values.is_empty() {
        return CBMT_ERROR_PROOF_ROOT;
    }
    let leaf_offset = leaves_count as u32 - 1;

    // Build (index, node) pairs by extracting the leaves at the proof's
    // tree positions.
    let mut pairs: Vec<CbmtNodePair> = Vec::with_capacity(proof.indices.values.len());
    for &tree_idx in &proof.indices.values {
        if tree_idx < leaf_offset {
            return CBMT_ERROR_PROOF_ROOT;
        }
        let leaf_pos = (tree_idx - leaf_offset) as usize;
        if leaf_pos >= leaves_count {
            return CBMT_ERROR_PROOF_ROOT;
        }
        pairs.push(CbmtNodePair {
            index: tree_idx,
            node: leaves.nodes[leaf_pos].clone(),
        });
    }

    cbmt_simple_bubble_sort(&mut pairs, cbmt_node_pair_reverse_cmp);

    let mut queue: std::collections::VecDeque<CbmtNodePair> = pairs.into_iter().collect();
    let mut lemmas_offset = 0usize;
    let mut ctx: () = ();

    while let Some(pair_current) = queue.pop_front() {
        let index = pair_current.index;
        let node = pair_current.node;

        if index == 0 {
            if proof.lemmas.len() == lemmas_offset && queue.is_empty() {
                if node.bytes == expected_root.bytes {
                    return 0;
                } else {
                    return CBMT_ERROR_VERIFY_FAILED;
                }
            } else {
                return CBMT_ERROR_PROOF_ROOT;
            }
        }

        let sibling_index = cbmt_sibling(index);
        let mut sibling: Option<CbmtNode> = None;
        if let Some(front) = queue.front() {
            if front.index == sibling_index {
                let popped = queue.pop_front().unwrap();
                sibling = Some(popped.node);
            }
        }
        if sibling.is_none() && lemmas_offset < proof.lemmas.len() {
            sibling = Some(proof.lemmas[lemmas_offset].clone());
            lemmas_offset += 1;
        }

        if let Some(sib) = sibling {
            let parent = if cbmt_is_left(index) {
                merge(Some(&mut ctx), &node, &sib)
            } else {
                merge(Some(&mut ctx), &sib, &node)
            };
            queue.push_back(CbmtNodePair {
                index: cbmt_parent(index),
                node: parent,
            });
        }
    }
    CBMT_ERROR_PROOF_ROOT
}

pub fn cbmt_build_merkle_root(
    leaves: &CbmtLeaves,
    merge: fn(Option<&mut ()>, &CbmtNode, &CbmtNode) -> CbmtNode,
) -> Result<CbmtNode, i32> {
    let length = leaves.nodes.len();
    if length == 0 {
        return Ok(CbmtNode {
            bytes: [0; CBMT_NODE_SIZE],
        });
    }

    let capacity = (length + 1) >> 1;
    let mut queue: std::collections::VecDeque<CbmtNode> =
        std::collections::VecDeque::with_capacity(capacity);

    let mut ctx: () = ();
    let mut i = length as isize - 1;
    while i > 0 {
        let left = &leaves.nodes[(i - 1) as usize];
        let right = &leaves.nodes[i as usize];
        let merged = merge(Some(&mut ctx), left, right);
        queue.push_back(merged);
        i -= 2;
    }
    if length % 2 == 1 {
        queue.push_front(leaves.nodes[0].clone());
    }

    while queue.len() > 1 {
        let right = queue.pop_front().ok_or(CBMT_ERROR_QUEUE_EMPTY)?;
        let left = queue.pop_front().ok_or(CBMT_ERROR_QUEUE_EMPTY)?;
        let merged = merge(Some(&mut ctx), &left, &right);
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
        let dest_idx = offset + i;
        tree.nodes[dest_idx] = leaves.nodes[i].clone();
    }

    let mut ctx: () = ();
    for i in 0..leaves.nodes.len() - 1 {
        let rev_idx = leaves.nodes.len() - 2 - i;
        let left = tree.nodes[(rev_idx << 1) + 1].clone();
        let right = tree.nodes[(rev_idx << 1) + 2].clone();
        tree.nodes[rev_idx] = merge(Some(&mut ctx), &left, &right);
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
    // Build a merkle tree internally using a generic ctx-aware merge.
    if leaves.nodes.is_empty() {
        return CBMT_ERROR_BUILD_PROOF;
    }
    let length = leaves.nodes.len() * 2 - 1;
    let mut nodes = vec![CbmtNode::default(); length];
    let offset = leaves.nodes.len() - 1;
    for i in 0..leaves.nodes.len() {
        nodes[offset + i] = leaves.nodes[i].clone();
    }
    for i in 0..leaves.nodes.len() - 1 {
        let rev_idx = leaves.nodes.len() - 2 - i;
        let left = nodes[(rev_idx << 1) + 1].clone();
        let right = nodes[(rev_idx << 1) + 2].clone();
        nodes[rev_idx] = merge(merge_ctx, &left, &right);
    }
    let tree = CbmtTree {
        nodes,
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
