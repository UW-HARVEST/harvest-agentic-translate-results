use core::mem::size_of;
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
#[derive(Debug, Clone, Default)]
pub struct CbmtIndices {
    pub values: Vec<u32>,
    pub capacity: usize,
}
#[derive(Debug, Clone, Default)]
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
#[derive(Debug, Clone, Default)]
pub struct CbmtLeaves {
    pub nodes: Vec<CbmtNode>,
}
#[derive(Debug, Default)]
pub struct CbmtQueue<'a> {
    pub buffer: CbmtBuffer<'a>,
    pub width: usize,
    pub length: usize,
    pub capacity: usize,
    pub tail: usize,
    pub head: usize,
}
#[derive(Debug, Clone, Default)]
pub struct CbmtNodePair {
    pub index: u32,
    pub node: CbmtNode,
}

// Type alias for the node merge function.
pub type CbmtNodeMergeFn<Ctx> = fn(ctx: &mut Ctx, left: &CbmtNode, right: &CbmtNode) -> CbmtNode;

// ----- helper inline-ish functions matching the C macros -----
#[inline]
fn is_left(index: u32) -> bool {
    (index & 1) == 1
}
#[inline]
fn parent(index: u32) -> u32 {
    if index == 0 {
        0
    } else {
        (index - 1) >> 1
    }
}
#[inline]
fn sibling(index: u32) -> u32 {
    if index == 0 {
        0
    } else {
        ((index + 1) ^ 1) - 1
    }
}

// ----- public functions -----

pub fn cbmt_universal_swap(left: &mut [u8], right: &mut [u8], width: usize) {
    for i in 0..width {
        std::mem::swap(&mut left[i], &mut right[i]);
    }
}

pub fn cbmt_simple_bubble_sort<T>(slice: &mut [T], cmp: fn(&T, &T) -> i32) {
    let len = slice.len();
    if len == 0 {
        return;
    }
    for i in 0..(len - 1) {
        for j in (i + 1)..len {
            if cmp(&slice[i], &slice[j]) > 0 {
                slice.swap(i, j);
            }
        }
    }
}

pub fn cbmt_uint32_reverse_cmp(left: &u32, right: &u32) -> i32 {
    // Mirrors C: return right - left as int (with possible wrap)
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
    // Mirrors C with CBMT_NODE_I32: int32_t left - int32_t right (le bytes)
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

    // Initial queue values translated to internal tree-indices (descending).
    let mut translated: Vec<u32> = leaf_indices
        .values
        .iter()
        .map(|&v| v + (leaves_count - 1))
        .collect();
    cbmt_simple_bubble_sort(&mut translated, cbmt_uint32_reverse_cmp);
    let first_value = translated[0];
    if first_value >= (leaves_count << 1) - 1 {
        return Err(CBMT_ERROR_BUILD_PROOF);
    }

    let mut queue: VecDeque<u32> = translated.into_iter().collect();
    let mut lemmas: Vec<CbmtNode> = Vec::new();
    while !queue.is_empty() {
        let index = queue.pop_front().unwrap();
        if index == 0 {
            if !queue.is_empty() {
                return Err(CBMT_FATAL_BUILD_PROOF);
            }
            break;
        }
        let sib = sibling(index);
        let front_match = queue.front().map(|&x| x == sib).unwrap_or(false);
        if front_match {
            queue.pop_front();
        } else {
            lemmas.push(tree.nodes[sib as usize].clone());
        }
        let p = parent(index);
        if p != 0 {
            queue.push_back(p);
        }
    }

    // Build indices: translated values reordered so that tree.nodes[indices[i]] is sorted ascending.
    let mut indices_values: Vec<u32> = leaf_indices
        .values
        .iter()
        .map(|&v| v + (leaves_count - 1))
        .collect();

    let n = indices_values.len();
    if n >= 1 {
        for i in 0..n.saturating_sub(1) {
            for j in (i + 1)..n {
                let li = indices_values[i] as usize;
                let ri = indices_values[j] as usize;
                let order = cbmt_node_cmp(&tree.nodes[li], &tree.nodes[ri]);
                if order > 0 {
                    indices_values[i] = ri as u32;
                    indices_values[j] = li as u32;
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
        CbmtNode::default()
    } else {
        tree.nodes[0].clone()
    }
}

fn proof_root_inner<F: FnMut(&CbmtNode, &CbmtNode) -> CbmtNode>(
    proof: &CbmtProof,
    leaves: &CbmtLeaves,
    mut merge: F,
) -> Result<CbmtNode, i32> {
    if leaves.nodes.len() != proof.indices.values.len() || leaves.nodes.is_empty() {
        return Err(CBMT_ERROR_PROOF_ROOT);
    }

    let mut leaves_clone: Vec<CbmtNode> = leaves.nodes.clone();
    cbmt_simple_bubble_sort(&mut leaves_clone, cbmt_node_cmp);

    let mut pairs: Vec<CbmtNodePair> = (0..leaves.nodes.len())
        .map(|i| CbmtNodePair {
            index: proof.indices.values[i],
            node: leaves_clone[i].clone(),
        })
        .collect();
    cbmt_simple_bubble_sort(&mut pairs, cbmt_node_pair_reverse_cmp);

    let mut queue: VecDeque<CbmtNodePair> = pairs.into_iter().collect();
    let mut lemmas_offset: usize = 0;
    while !queue.is_empty() {
        let pair_current = queue.pop_front().unwrap();
        let index = pair_current.index;
        let node = pair_current.node;

        if index == 0 {
            if proof.lemmas.len() == lemmas_offset && queue.is_empty() {
                return Ok(node);
            } else {
                return Err(CBMT_ERROR_PROOF_ROOT);
            }
        }

        let sib = sibling(index);
        let mut sibling_node: Option<CbmtNode> = None;
        if let Some(pair_front) = queue.front() {
            if pair_front.index == sib {
                let p = queue.pop_front().unwrap();
                sibling_node = Some(p.node);
            }
        }
        if sibling_node.is_none() && lemmas_offset < proof.lemmas.len() {
            sibling_node = Some(proof.lemmas[lemmas_offset].clone());
            lemmas_offset += 1;
        }
        if let Some(sib_node) = sibling_node {
            let parent_node = if is_left(index) {
                merge(&node, &sib_node)
            } else {
                merge(&sib_node, &node)
            };
            queue.push_back(CbmtNodePair {
                index: parent(index),
                node: parent_node,
            });
        }
    }
    // Mirrors the C return 0 with root unset behavior.
    Ok(CbmtNode::default())
}

pub fn cbmt_proof_root<Ctx>(
    proof: &CbmtProof,
    root: &mut CbmtNode,
    leaves: &CbmtLeaves,
    merge: CbmtNodeMergeFn<Ctx>,
    merge_ctx: &mut Ctx,
    nodes_buffer: CbmtBuffer,
    pairs_buffer: CbmtBuffer,
) -> i32 {
    if leaves.nodes.len() * size_of::<CbmtNode>() > nodes_buffer.capacity {
        return CBMT_ERROR_OVER_CAPACITY;
    }
    if leaves.nodes.len() * size_of::<CbmtNodePair>() > pairs_buffer.capacity {
        return CBMT_ERROR_OVER_CAPACITY;
    }
    match proof_root_inner(proof, leaves, |l, r| merge(merge_ctx, l, r)) {
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
    let target = match proof_root_inner(proof, leaves, |l, r| merge(None, l, r)) {
        Ok(n) => n,
        Err(e) => return e,
    };
    if target.bytes != expected_root.bytes {
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

    let mut i = length as i64 - 1;
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
        .map(Ok)
        .unwrap_or(Err(CBMT_ERROR_QUEUE_EMPTY))
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
    for i in 0..(leaves.nodes.len() - 1) {
        let rev_idx = leaves.nodes.len() - 2 - i;
        let left = tree.nodes[(rev_idx << 1) + 1].clone();
        let right = tree.nodes[(rev_idx << 1) + 2].clone();
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
    nodes_buffer: CbmtBuffer,
    indices_buffer: CbmtBuffer,
    lemmas_buffer: CbmtBuffer,
) -> i32 {
    // Build tree first, mirroring cbmt_build_merkle_tree but with the generic merge ctx.
    let mut tree = CbmtTree::default();
    if leaves.nodes.is_empty() {
        tree.length = 0;
    } else {
        let length = leaves.nodes.len() * 2 - 1;
        if length * size_of::<CbmtNode>() > nodes_buffer.capacity {
            return CBMT_ERROR_OVER_CAPACITY;
        }
        tree.nodes = vec![CbmtNode::default(); length];
        tree.length = length;
        tree.capacity = length;
        let offset = leaves.nodes.len() - 1;
        for i in 0..leaves.nodes.len() {
            tree.nodes[offset + i] = leaves.nodes[i].clone();
        }
        for i in 0..(leaves.nodes.len() - 1) {
            let rev_idx = leaves.nodes.len() - 2 - i;
            let left = tree.nodes[(rev_idx << 1) + 1].clone();
            let right = tree.nodes[(rev_idx << 1) + 2].clone();
            tree.nodes[rev_idx] = merge(merge_ctx, &left, &right);
        }
    }

    if leaf_indices.values.len() * size_of::<u32>() > indices_buffer.capacity {
        return CBMT_ERROR_OVER_CAPACITY;
    }

    match cbmt_tree_build_proof(&tree, leaf_indices) {
        Ok(p) => {
            if p.lemmas.len() * size_of::<CbmtNode>() > lemmas_buffer.capacity {
                return CBMT_ERROR_OVER_CAPACITY;
            }
            *proof = p;
            0
        }
        Err(e) => e,
    }
}
