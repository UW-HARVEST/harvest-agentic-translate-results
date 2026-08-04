use std::cmp::Ordering;
use std::collections::VecDeque;
use std::mem::size_of;

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
pub fn cbmt_universal_swap(left: &mut [u8], right: &mut [u8], width: usize) {
    if width == 0 {
        return;
    }
    left[..width].swap_with_slice(&mut right[..width]);
}
pub fn cbmt_simple_bubble_sort<T>(slice: &mut [T], cmp: fn(&T, &T) -> i32) {
    if slice.len() < 2 {
        return;
    }
    for i in 0..(slice.len() - 1) {
        for j in (i + 1)..slice.len() {
            if cmp(&slice[i], &slice[j]) > 0 {
                slice.swap(i, j);
            }
        }
    }
}
pub fn cbmt_uint32_reverse_cmp(left: &u32, right: &u32) -> i32 {
    cmp_to_i32(right.cmp(left))
}
pub fn cbmt_buffer_init<'a>(buffer: &mut CbmtBuffer<'a>, data: &'a mut [u8]) {
    buffer.data = data;
    buffer.capacity = buffer.data.len();
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
    if item.len() < queue.width {
        return CBMT_ERROR_INVALID_CAPACITY;
    }
    let start = queue.head * queue.width;
    let end = start + queue.width;
    queue.buffer.data[start..end].copy_from_slice(&item[..queue.width]);
    queue.head = (queue.head + 1) % queue.capacity;
    queue.length += 1;
    0
}
pub fn cbmt_queue_push_front(queue: &mut CbmtQueue, item: &[u8]) -> i32 {
    if queue.length >= queue.capacity {
        return CBMT_ERROR_OVER_CAPACITY;
    }
    if item.len() < queue.width {
        return CBMT_ERROR_INVALID_CAPACITY;
    }
    queue.tail = (queue.tail + queue.capacity - 1) % queue.capacity;
    let start = queue.tail * queue.width;
    let end = start + queue.width;
    queue.buffer.data[start..end].copy_from_slice(&item[..queue.width]);
    queue.length += 1;
    0
}
pub fn cbmt_queue_pop_front(queue: &mut CbmtQueue, item: &mut [u8]) -> i32 {
    if queue.length == 0 {
        return CBMT_ERROR_QUEUE_EMPTY;
    }
    if item.len() < queue.width {
        return CBMT_ERROR_INVALID_CAPACITY;
    }
    let start = queue.tail * queue.width;
    let end = start + queue.width;
    item[..queue.width].copy_from_slice(&queue.buffer.data[start..end]);
    queue.tail = (queue.tail + 1) % queue.capacity;
    queue.length -= 1;
    0
}
pub fn cbmt_queue_front<'a>(queue: &'a CbmtQueue<'a>) -> Option<&'a [u8]> {
    if queue.length == 0 {
        return None;
    }
    let start = queue.tail * queue.width;
    let end = start + queue.width;
    Some(&queue.buffer.data[start..end])
}
pub fn cbmt_node_copy(dest: &mut CbmtNode, src: &CbmtNode) {
    dest.bytes.copy_from_slice(&src.bytes);
}
pub fn cbmt_node_cmp(left: &CbmtNode, right: &CbmtNode) -> i32 {
    cmp_to_i32(node_i32(left).cmp(&node_i32(right)))
}
pub fn cbmt_node_pair_reverse_cmp(left: &CbmtNodePair, right: &CbmtNodePair) -> i32 {
    cmp_to_i32(right.index.cmp(&left.index))
}
pub fn cbmt_tree_build_proof(
    tree: &CbmtTree,
    leaf_indices: &CbmtIndices,
) -> Result<CbmtProof, i32> {
    if tree.length == 0 || leaf_indices.values.is_empty() {
        return Err(CBMT_ERROR_BUILD_PROOF);
    }

    let leaves_count = ((tree.length >> 1) + 1) as u32;
    let mut queue = leaf_indices
        .values
        .iter()
        .map(|value| value + (leaves_count - 1))
        .collect::<Vec<u32>>();
    cbmt_simple_bubble_sort(&mut queue, cbmt_uint32_reverse_cmp);

    if queue
        .first()
        .is_some_and(|value| *value >= ((leaves_count << 1) - 1))
    {
        return Err(CBMT_ERROR_BUILD_PROOF);
    }

    let mut queue = VecDeque::from(queue);
    let mut lemmas = Vec::new();
    while let Some(index) = queue.pop_front() {
        if index == 0 {
            if !queue.is_empty() {
                return Err(CBMT_FATAL_BUILD_PROOF);
            }
            break;
        }

        let sibling = cbmt_sibling(index);
        if queue.front().copied() == Some(sibling) {
            queue.pop_front();
        } else {
            let sibling_index = sibling as usize;
            if sibling_index >= tree.nodes.len() {
                return Err(CBMT_ERROR_BUILD_PROOF);
            }
            lemmas.push(tree.nodes[sibling_index].clone());
        }

        let parent = cbmt_parent(index);
        if parent != 0 {
            queue.push_back(parent);
        }
    }

    let mut indices = leaf_indices
        .values
        .iter()
        .map(|value| value + (leaves_count - 1))
        .collect::<Vec<u32>>();
    for i in 0..indices.len().saturating_sub(1) {
        for j in (i + 1)..indices.len() {
            let left_index = indices[i] as usize;
            let right_index = indices[j] as usize;
            if cbmt_node_cmp(&tree.nodes[left_index], &tree.nodes[right_index]) > 0 {
                indices.swap(i, j);
            }
        }
    }

    Ok(CbmtProof {
        indices: CbmtIndices {
            capacity: indices.len(),
            values: indices,
        },
        lemmas,
    })
}
pub fn cbmt_tree_root(tree: &CbmtTree) -> CbmtNode {
    tree.nodes.first().cloned().unwrap_or_default()
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
    cbmt_proof_root_impl(
        proof,
        root,
        leaves,
        |left, right| merge(merge_ctx, left, right),
        nodes_buffer.capacity,
        pairs_buffer.capacity,
    )
}
pub fn cbmt_proof_verify(
    proof: &CbmtProof,
    expected_root: &CbmtNode,
    leaves: &CbmtLeaves,
    merge: fn(Option<&mut ()>, &CbmtNode, &CbmtNode) -> CbmtNode,
) -> i32 {
    let mut root = CbmtNode::default();
    let nodes_storage = vec![0u8; leaves.nodes.len().saturating_mul(size_of::<CbmtNode>())];
    let pairs_storage = vec![0u8; leaves.nodes.len().saturating_mul(size_of::<CbmtNodePair>())];
    let nodes_capacity = nodes_storage.len();
    let pairs_capacity = pairs_storage.len();
    let ret = cbmt_proof_root_impl(
        proof,
        &mut root,
        leaves,
        |left, right| merge(None, left, right),
        nodes_capacity,
        pairs_capacity,
    );
    if ret != 0 {
        return ret;
    }
    if root.bytes != expected_root.bytes {
        return CBMT_ERROR_VERIFY_FAILED;
    }
    0
}
pub fn cbmt_build_merkle_root(
    leaves: &CbmtLeaves,
    merge: fn(Option<&mut ()>, &CbmtNode, &CbmtNode) -> CbmtNode,
) -> Result<CbmtNode, i32> {
    cbmt_build_merkle_root_impl(leaves, |left, right| merge(None, left, right))
}
pub fn cbmt_build_merkle_tree(
    tree: &mut CbmtTree,
    leaves: &CbmtLeaves,
    merge: fn(Option<&mut ()>, &CbmtNode, &CbmtNode) -> CbmtNode,
) -> i32 {
    cbmt_build_merkle_tree_impl(tree, leaves, |left, right| merge(None, left, right))
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
    let required_tree_nodes = leaves.nodes.len().saturating_mul(2).saturating_sub(1);
    if required_tree_nodes.saturating_mul(size_of::<CbmtNode>()) > nodes_buffer.capacity {
        return CBMT_ERROR_OVER_CAPACITY;
    }
    if leaf_indices.values.len().saturating_mul(size_of::<u32>()) > indices_buffer.capacity {
        return CBMT_ERROR_OVER_CAPACITY;
    }

    let mut tree = CbmtTree::default();
    let ret = cbmt_build_merkle_tree_impl(&mut tree, leaves, |left, right| {
        merge(merge_ctx, left, right)
    });
    if ret != 0 {
        return ret;
    }

    match cbmt_tree_build_proof(&tree, leaf_indices) {
        Ok(built_proof) => {
            if built_proof
                .lemmas
                .len()
                .saturating_mul(size_of::<CbmtNode>())
                > lemmas_buffer.capacity
            {
                return CBMT_ERROR_OVER_CAPACITY;
            }
            *proof = built_proof;
            0
        }
        Err(err) => err,
    }
}

fn cmp_to_i32(ordering: Ordering) -> i32 {
    match ordering {
        Ordering::Less => -1,
        Ordering::Equal => 0,
        Ordering::Greater => 1,
    }
}

fn cbmt_is_left(index: u32) -> bool {
    (index & 1) == 1
}

fn cbmt_parent(index: u32) -> u32 {
    if index == 0 {
        0
    } else {
        (index - 1) >> 1
    }
}

fn cbmt_sibling(index: u32) -> u32 {
    if index == 0 {
        0
    } else {
        ((index + 1) ^ 1) - 1
    }
}

fn node_i32(node: &CbmtNode) -> i32 {
    i32::from_le_bytes(node.bytes)
}

fn cbmt_proof_root_impl<F>(
    proof: &CbmtProof,
    root: &mut CbmtNode,
    leaves: &CbmtLeaves,
    mut merge: F,
    nodes_capacity: usize,
    pairs_capacity: usize,
) -> i32
where
    F: FnMut(&CbmtNode, &CbmtNode) -> CbmtNode,
{
    if leaves.nodes.len().saturating_mul(size_of::<CbmtNode>()) > nodes_capacity {
        return CBMT_ERROR_OVER_CAPACITY;
    }
    if leaves.nodes.len().saturating_mul(size_of::<CbmtNodePair>()) > pairs_capacity {
        return CBMT_ERROR_OVER_CAPACITY;
    }
    if leaves.nodes.len() != proof.indices.values.len() || leaves.nodes.is_empty() {
        return CBMT_ERROR_PROOF_ROOT;
    }

    let mut leaves_clone = leaves.nodes.clone();
    cbmt_simple_bubble_sort(&mut leaves_clone, cbmt_node_cmp);

    let mut queue = leaves_clone
        .into_iter()
        .enumerate()
        .map(|(idx, node)| CbmtNodePair {
            index: proof.indices.values[idx],
            node,
        })
        .collect::<Vec<CbmtNodePair>>();
    cbmt_simple_bubble_sort(&mut queue, cbmt_node_pair_reverse_cmp);

    let mut queue = VecDeque::from(queue);
    let mut lemmas_offset = 0usize;

    while let Some(pair_current) = queue.pop_front() {
        let index = pair_current.index;
        let node = pair_current.node;

        if index == 0 {
            if proof.lemmas.len() == lemmas_offset && queue.is_empty() {
                *root = node;
                return 0;
            }
            return CBMT_ERROR_PROOF_ROOT;
        }

        let sibling_index = cbmt_sibling(index);
        let sibling = if queue.front().map(|pair| pair.index) == Some(sibling_index) {
            queue.pop_front().map(|pair| pair.node)
        } else if lemmas_offset < proof.lemmas.len() {
            let node = proof.lemmas[lemmas_offset].clone();
            lemmas_offset += 1;
            Some(node)
        } else {
            None
        };

        if let Some(sibling) = sibling {
            let parent = if cbmt_is_left(index) {
                merge(&node, &sibling)
            } else {
                merge(&sibling, &node)
            };
            queue.push_back(CbmtNodePair {
                index: cbmt_parent(index),
                node: parent,
            });
        }
    }

    0
}

fn cbmt_build_merkle_root_impl<F>(leaves: &CbmtLeaves, mut merge: F) -> Result<CbmtNode, i32>
where
    F: FnMut(&CbmtNode, &CbmtNode) -> CbmtNode,
{
    let length = leaves.nodes.len();
    if length == 0 {
        return Ok(CbmtNode::default());
    }

    let mut queue = VecDeque::new();
    let mut i = length;
    while i > 1 {
        let left = &leaves.nodes[i - 2];
        let right = &leaves.nodes[i - 1];
        queue.push_back(merge(left, right));
        i = i.saturating_sub(2);
    }
    if length % 2 == 1 {
        queue.push_front(leaves.nodes[0].clone());
    }

    while queue.len() > 1 {
        let right = queue.pop_front().ok_or(CBMT_ERROR_QUEUE_EMPTY)?;
        let left = queue.pop_front().ok_or(CBMT_ERROR_QUEUE_EMPTY)?;
        queue.push_back(merge(&left, &right));
    }

    queue.pop_front().ok_or(CBMT_ERROR_QUEUE_EMPTY)
}

fn cbmt_build_merkle_tree_impl<F>(tree: &mut CbmtTree, leaves: &CbmtLeaves, mut merge: F) -> i32
where
    F: FnMut(&CbmtNode, &CbmtNode) -> CbmtNode,
{
    if leaves.nodes.is_empty() {
        tree.nodes.clear();
        tree.length = 0;
        tree.capacity = 0;
        return 0;
    }

    let length = leaves.nodes.len() * 2 - 1;
    tree.nodes = vec![CbmtNode::default(); length];
    tree.length = length;
    tree.capacity = length;

    let offset = leaves.nodes.len() - 1;
    for (idx, node) in leaves.nodes.iter().enumerate() {
        tree.nodes[offset + idx] = node.clone();
    }

    for i in 0..(leaves.nodes.len() - 1) {
        let rev_idx = leaves.nodes.len() - 2 - i;
        let left = tree.nodes[(rev_idx << 1) + 1].clone();
        let right = tree.nodes[(rev_idx << 1) + 2].clone();
        tree.nodes[rev_idx] = merge(&left, &right);
    }
    0
}
