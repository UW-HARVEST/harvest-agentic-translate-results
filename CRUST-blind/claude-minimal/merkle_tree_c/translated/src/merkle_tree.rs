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
    for i in 0..width {
        std::mem::swap(&mut left[i], &mut right[i]);
    }
}

pub fn cbmt_simple_bubble_sort<T>(slice: &mut [T], cmp: fn(&T, &T) -> i32) {
    let length = slice.len();
    if length < 2 {
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
    // Mirror C: returns `right - left` cast to `int`. Use wrapping_sub so the
    // bit pattern matches, then reinterpret as i32.
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
    // Mirror the default (non-CBMT_NODE_I32) byte-wise comparison from C.
    for i in 0..CBMT_NODE_SIZE {
        let l = left.bytes[i] as i32;
        let r = right.bytes[i] as i32;
        let diff = l - r;
        if diff != 0 {
            return diff;
        }
    }
    0
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
    let leaves_count = ((tree.length >> 1) + 1) as u32;

    // Translate the leaf indices into internal tree indices, then sort
    // descending so the deepest nodes are processed first.
    let mut queue: VecDeque<u32> = VecDeque::with_capacity(leaf_indices.values.len());
    for &v in &leaf_indices.values {
        queue.push_back(v + (leaves_count - 1));
    }
    let mut sorted: Vec<u32> = queue.drain(..).collect();
    sorted.sort_by(|a, b| b.cmp(a));
    for v in sorted {
        queue.push_back(v);
    }

    let first_value = match queue.front() {
        Some(&v) => v,
        None => return Err(CBMT_ERROR_BUILD_PROOF),
    };
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
        let consume_sibling = matches!(queue.front(), Some(&f) if f == sibling);
        if consume_sibling {
            queue.pop_front();
        } else {
            lemmas.push(tree.nodes[sibling as usize].clone());
        }

        let parent = cbmt_parent(index);
        if parent != 0 {
            queue.push_back(parent);
        }
    }

    // Build proof.indices: original leaf indices remapped, then sorted by the
    // node value at the corresponding tree position.
    let mut indices_values: Vec<u32> = leaf_indices
        .values
        .iter()
        .map(|&v| v + (leaves_count - 1))
        .collect();

    let len = indices_values.len();
    if len > 1 {
        for i in 0..len - 1 {
            for j in i + 1..len {
                let li = indices_values[i] as usize;
                let ri = indices_values[j] as usize;
                let order = cbmt_node_cmp(&tree.nodes[li], &tree.nodes[ri]);
                if order > 0 {
                    indices_values.swap(i, j);
                }
            }
        }
    }

    let cap = indices_values.len();
    Ok(CbmtProof {
        indices: CbmtIndices {
            values: indices_values,
            capacity: cap,
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

/// Shared implementation for proof_root / proof_verify. Reconstructs the tree
/// root by repeatedly merging known nodes with their siblings (drawn from the
/// queue or from the lemma list).
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

    // Sort cloned leaves to align with the proof's index order (which the
    // builder sorted by node value).
    let mut leaves_clone: Vec<CbmtNode> = leaves.nodes.clone();
    leaves_clone.sort_by(|a, b| cbmt_node_cmp(a, b).cmp(&0));

    // Pair each sorted leaf with its tree index, then sort descending by
    // index so deeper positions are processed first.
    let mut pairs: Vec<CbmtNodePair> = (0..leaves_clone.len())
        .map(|i| CbmtNodePair {
            index: proof.indices.values[i],
            node: leaves_clone[i].clone(),
        })
        .collect();
    pairs.sort_by(|a, b| b.index.cmp(&a.index));
    let mut queue: VecDeque<CbmtNodePair> = pairs.into_iter().collect();

    let mut lemmas_offset = 0usize;
    while let Some(pair_current) = queue.pop_front() {
        let index = pair_current.index;
        if index == 0 {
            if proof.lemmas.len() == lemmas_offset && queue.is_empty() {
                return Ok(pair_current.node);
            } else {
                return Err(CBMT_ERROR_PROOF_ROOT);
            }
        }

        let sibling_idx = cbmt_sibling(index);
        let sibling: Option<CbmtNode> = match queue.front() {
            Some(p) if p.index == sibling_idx => {
                let s = queue.pop_front().unwrap();
                Some(s.node)
            }
            _ => {
                if lemmas_offset < proof.lemmas.len() {
                    let n = proof.lemmas[lemmas_offset].clone();
                    lemmas_offset += 1;
                    Some(n)
                } else {
                    None
                }
            }
        };

        if let Some(sib) = sibling {
            let parent_node = if cbmt_is_left(index) {
                merge(&pair_current.node, &sib)
            } else {
                merge(&sib, &pair_current.node)
            };
            queue.push_back(CbmtNodePair {
                index: cbmt_parent(index),
                node: parent_node,
            });
        }
    }

    // Mirror the C path that falls out of the loop without setting root —
    // shouldn't be reached for a well-formed proof.
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
    let result = proof_root_compute(proof, leaves, |l, r| merge(&mut *merge_ctx, l, r));
    match result {
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
    let target_root = match proof_root_compute(proof, leaves, |l, r| merge(None, l, r)) {
        Ok(n) => n,
        Err(e) => return e,
    };
    for i in 0..CBMT_NODE_SIZE {
        if target_root.bytes[i] != expected_root.bytes[i] {
            return CBMT_ERROR_VERIFY_FAILED;
        }
    }
    0
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

    let mut queue: VecDeque<CbmtNode> = VecDeque::with_capacity((length + 1) >> 1);

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
        let right = queue.pop_front().unwrap();
        let left = queue.pop_front().unwrap();
        let merged = merge(None, &left, &right);
        queue.push_back(merged);
    }

    queue
        .pop_front()
        .ok_or(CBMT_ERROR_QUEUE_EMPTY)
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
    // Build the tree inline: cbmt_build_merkle_tree uses a different fn-pointer
    // signature (no Ctx), so we can't delegate to it here.
    let mut tree = CbmtTree::default();
    if !leaves.nodes.is_empty() {
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
            tree.nodes[rev_idx] = merge(&mut *merge_ctx, &left, &right);
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
