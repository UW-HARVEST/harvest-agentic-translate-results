use std::cmp::Ordering;
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

#[derive(Debug, Clone, Default, PartialEq, Eq)]
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

pub type CbmtNodeMergeFn<Ctx> = fn(ctx: &mut Ctx, left: &CbmtNode, right: &CbmtNode) -> CbmtNode;

fn cbmt_is_left(index: u32) -> bool {
    (index & 1) == 1
}

fn cbmt_parent(index: u32) -> u32 {
    if index == 0 { 0 } else { (index - 1) >> 1 }
}

fn cbmt_sibling(index: u32) -> u32 {
    if index == 0 { 0 } else { ((index + 1) ^ 1) - 1 }
}

fn queue_range(queue: &CbmtQueue, index: usize) -> std::ops::Range<usize> {
    let start = index * queue.width;
    start..start + queue.width
}

fn order_to_i32(order: Ordering) -> i32 {
    match order {
        Ordering::Less => -1,
        Ordering::Equal => 0,
        Ordering::Greater => 1,
    }
}

pub fn cbmt_universal_swap(left: &mut [u8], right: &mut [u8], width: usize) {
    assert!(left.len() >= width);
    assert!(right.len() >= width);

    let mut offset = 0;
    while offset < width {
        let end = usize::min(offset + 128, width);
        let len = end - offset;
        let mut tmp = [0u8; 128];
        tmp[..len].copy_from_slice(&left[offset..end]);
        left[offset..end].copy_from_slice(&right[offset..end]);
        right[offset..end].copy_from_slice(&tmp[..len]);
        offset = end;
    }
}

pub fn cbmt_simple_bubble_sort<T>(slice: &mut [T], cmp: fn(&T, &T) -> i32) {
    if slice.len() < 2 {
        return;
    }

    for i in 0..slice.len() - 1 {
        for j in i + 1..slice.len() {
            if cmp(&slice[i], &slice[j]) > 0 {
                slice.swap(i, j);
            }
        }
    }
}

pub fn cbmt_uint32_reverse_cmp(left: &u32, right: &u32) -> i32 {
    order_to_i32(right.cmp(left))
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
    if capacity.saturating_mul(width) > buffer.capacity {
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
    if queue.length >= queue.capacity || item.len() < queue.width {
        return CBMT_ERROR_OVER_CAPACITY;
    }

    let range = queue_range(queue, queue.head);
    queue.buffer.data[range].copy_from_slice(&item[..queue.width]);
    queue.head = (queue.head + 1) % queue.capacity;
    queue.length += 1;
    0
}

pub fn cbmt_queue_push_front(queue: &mut CbmtQueue, item: &[u8]) -> i32 {
    if queue.length >= queue.capacity || item.len() < queue.width {
        return CBMT_ERROR_OVER_CAPACITY;
    }

    queue.tail = (queue.tail + queue.capacity - 1) % queue.capacity;
    let range = queue_range(queue, queue.tail);
    queue.buffer.data[range].copy_from_slice(&item[..queue.width]);
    queue.length += 1;
    0
}

pub fn cbmt_queue_pop_front(queue: &mut CbmtQueue, item: &mut [u8]) -> i32 {
    if queue.length == 0 {
        return CBMT_ERROR_QUEUE_EMPTY;
    }
    if item.len() < queue.width {
        return CBMT_ERROR_OVER_CAPACITY;
    }

    let range = queue_range(queue, queue.tail);
    item[..queue.width].copy_from_slice(&queue.buffer.data[range]);
    queue.tail = (queue.tail + 1) % queue.capacity;
    queue.length -= 1;
    0
}

pub fn cbmt_queue_front<'a>(queue: &'a CbmtQueue<'a>) -> Option<&'a [u8]> {
    if queue.length == 0 {
        return None;
    }

    let range = queue_range(queue, queue.tail);
    Some(&queue.buffer.data[range])
}

pub fn cbmt_node_copy(dest: &mut CbmtNode, src: &CbmtNode) {
    dest.bytes.copy_from_slice(&src.bytes);
}

pub fn cbmt_node_cmp(left: &CbmtNode, right: &CbmtNode) -> i32 {
    if CBMT_NODE_SIZE == 4 {
        let left_value = i32::from_le_bytes(left.bytes);
        let right_value = i32::from_le_bytes(right.bytes);
        left_value.wrapping_sub(right_value)
    } else {
        for i in 0..CBMT_NODE_SIZE {
            let cmp = i32::from(left.bytes[i]) - i32::from(right.bytes[i]);
            if cmp != 0 {
                return cmp;
            }
        }
        0
    }
}

pub fn cbmt_node_pair_reverse_cmp(left: &CbmtNodePair, right: &CbmtNodePair) -> i32 {
    order_to_i32(right.index.cmp(&left.index))
}

pub fn cbmt_tree_build_proof(
    tree: &CbmtTree,
    leaf_indices: &CbmtIndices,
) -> Result<CbmtProof, i32> {
    if tree.length == 0 || leaf_indices.values.is_empty() {
        return Err(CBMT_ERROR_BUILD_PROOF);
    }

    let leaves_count = ((tree.length >> 1) + 1) as u32;
    let mut queue: VecDeque<u32> = leaf_indices
        .values
        .iter()
        .map(|value| value + (leaves_count - 1))
        .collect();

    let mut sorted = queue.make_contiguous();
    cbmt_simple_bubble_sort(&mut sorted, cbmt_uint32_reverse_cmp);

    if queue
        .front()
        .copied()
        .is_some_and(|value| value >= ((leaves_count << 1) - 1))
    {
        return Err(CBMT_ERROR_BUILD_PROOF);
    }

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

    let mut indices = CbmtIndices {
        values: leaf_indices
            .values
            .iter()
            .map(|value| value + (leaves_count - 1))
            .collect(),
        capacity: leaf_indices.capacity,
    };
    if indices.values.len() > 1 {
        for i in 0..indices.values.len() - 1 {
            for j in i + 1..indices.values.len() {
                let left_index = indices.values[i] as usize;
                let right_index = indices.values[j] as usize;
                if cbmt_node_cmp(&tree.nodes[left_index], &tree.nodes[right_index]) > 0 {
                    indices.values.swap(i, j);
                }
            }
        }
    }

    Ok(CbmtProof { indices, lemmas })
}

pub fn cbmt_tree_root(tree: &CbmtTree) -> CbmtNode {
    if tree.length == 0 {
        CbmtNode::default()
    } else {
        tree.nodes[0].clone()
    }
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
    if leaves.nodes.len().saturating_mul(std::mem::size_of::<CbmtNode>()) > nodes_buffer.capacity {
        return CBMT_ERROR_OVER_CAPACITY;
    }
    if leaves.nodes.len().saturating_mul(std::mem::size_of::<CbmtNodePair>()) > pairs_buffer.capacity
    {
        return CBMT_ERROR_OVER_CAPACITY;
    }
    if leaves.nodes.len() != proof.indices.values.len() || leaves.nodes.is_empty() {
        return CBMT_ERROR_PROOF_ROOT;
    }

    let mut leaves_clone = leaves.nodes.clone();
    cbmt_simple_bubble_sort(&mut leaves_clone, cbmt_node_cmp);

    let mut queue: VecDeque<CbmtNodePair> = proof
        .indices
        .values
        .iter()
        .zip(leaves_clone.iter())
        .map(|(index, node)| CbmtNodePair {
            index: *index,
            node: node.clone(),
        })
        .collect();
    let mut sorted = queue.make_contiguous();
    cbmt_simple_bubble_sort(&mut sorted, cbmt_node_pair_reverse_cmp);

    let mut lemmas_offset = 0usize;
    while let Some(pair_current) = queue.pop_front() {
        let index = pair_current.index;
        let node = pair_current.node;

        if index == 0 {
            if proof.lemmas.len() == lemmas_offset && queue.is_empty() {
                cbmt_node_copy(root, &node);
                return 0;
            }
            return CBMT_ERROR_PROOF_ROOT;
        }

        let sibling = if queue
            .front()
            .is_some_and(|pair_front| pair_front.index == cbmt_sibling(index))
        {
            queue.pop_front().map(|pair| pair.node)
        } else if lemmas_offset < proof.lemmas.len() {
            let lemma = proof.lemmas[lemmas_offset].clone();
            lemmas_offset += 1;
            Some(lemma)
        } else {
            None
        };

        if let Some(sibling_node) = sibling {
            let parent = if cbmt_is_left(index) {
                merge(merge_ctx, &node, &sibling_node)
            } else {
                merge(merge_ctx, &sibling_node, &node)
            };
            queue.push_back(CbmtNodePair {
                index: cbmt_parent(index),
                node: parent,
            });
        }
    }

    0
}

pub fn cbmt_proof_verify(
    proof: &CbmtProof,
    expected_root: &CbmtNode,
    leaves: &CbmtLeaves,
    merge: fn(Option<&mut ()>, &CbmtNode, &CbmtNode) -> CbmtNode,
) -> i32 {
    let effective_leaves = if leaves.nodes.len() == proof.indices.values.len() {
        leaves.nodes.clone()
    } else if !leaves.nodes.is_empty() && leaves.nodes.len() >= proof.indices.values.len() {
        let offset = (leaves.nodes.len() - 1) as u32;
        let mut selected = Vec::with_capacity(proof.indices.values.len());
        for index in &proof.indices.values {
            if *index < offset {
                return CBMT_ERROR_PROOF_ROOT;
            }
            let leaf_index = (*index - offset) as usize;
            let Some(node) = leaves.nodes.get(leaf_index) else {
                return CBMT_ERROR_PROOF_ROOT;
            };
            selected.push(node.clone());
        }
        selected
    } else {
        return CBMT_ERROR_PROOF_ROOT;
    };

    if effective_leaves.is_empty() {
        return CBMT_ERROR_PROOF_ROOT;
    }

    let mut leaves_clone = effective_leaves;
    cbmt_simple_bubble_sort(&mut leaves_clone, cbmt_node_cmp);

    let mut queue: VecDeque<CbmtNodePair> = proof
        .indices
        .values
        .iter()
        .zip(leaves_clone.iter())
        .map(|(index, node)| CbmtNodePair {
            index: *index,
            node: node.clone(),
        })
        .collect();
    let sorted = queue.make_contiguous();
    cbmt_simple_bubble_sort(sorted, cbmt_node_pair_reverse_cmp);

    let mut lemmas_offset = 0usize;
    let mut target_root = None;
    while let Some(pair_current) = queue.pop_front() {
        let index = pair_current.index;
        let node = pair_current.node;

        if index == 0 {
            if proof.lemmas.len() == lemmas_offset && queue.is_empty() {
                target_root = Some(node);
                break;
            }
            return CBMT_ERROR_PROOF_ROOT;
        }

        let sibling = if queue
            .front()
            .is_some_and(|pair_front| pair_front.index == cbmt_sibling(index))
        {
            queue.pop_front().map(|pair| pair.node)
        } else if lemmas_offset < proof.lemmas.len() {
            let lemma = proof.lemmas[lemmas_offset].clone();
            lemmas_offset += 1;
            Some(lemma)
        } else {
            None
        };

        if let Some(sibling_node) = sibling {
            let parent = if cbmt_is_left(index) {
                merge(None, &node, &sibling_node)
            } else {
                merge(None, &sibling_node, &node)
            };
            queue.push_back(CbmtNodePair {
                index: cbmt_parent(index),
                node: parent,
            });
        }
    }

    let Some(target_root) = target_root else {
        return 0;
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

    let mut queue = VecDeque::with_capacity((length + 1) >> 1);
    let mut i = length;
    while i > 1 {
        let right_index = i - 1;
        let left_index = i - 2;
        queue.push_back(merge(None, &leaves.nodes[left_index], &leaves.nodes[right_index]));
        i = i.saturating_sub(2);
    }
    if length % 2 == 1 {
        queue.push_front(leaves.nodes[0].clone());
    }

    while queue.len() > 1 {
        let right = queue.pop_front().unwrap();
        let left = queue.pop_front().unwrap();
        queue.push_back(merge(None, &left, &right));
    }

    Ok(queue.pop_front().unwrap())
}

pub fn cbmt_build_merkle_tree(
    tree: &mut CbmtTree,
    leaves: &CbmtLeaves,
    merge: fn(Option<&mut ()>, &CbmtNode, &CbmtNode) -> CbmtNode,
) -> i32 {
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
    for (i, node) in leaves.nodes.iter().enumerate() {
        cbmt_node_copy(&mut tree.nodes[offset + i], node);
    }

    for i in 0..leaves.nodes.len() - 1 {
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
    _indices_buffer: CbmtBuffer,
    _lemmas_buffer: CbmtBuffer,
) -> i32 {
    let tree_length = if leaves.nodes.is_empty() {
        0
    } else {
        leaves.nodes.len() * 2 - 1
    };
    if tree_length.saturating_mul(std::mem::size_of::<CbmtNode>()) > nodes_buffer.capacity {
        return CBMT_ERROR_OVER_CAPACITY;
    }

    let mut tree = CbmtTree::default();
    if leaves.nodes.is_empty() {
        tree.nodes.clear();
        tree.length = 0;
        tree.capacity = 0;
    } else {
        tree.nodes = vec![CbmtNode::default(); tree_length];
        tree.length = tree_length;
        tree.capacity = tree_length;

        let offset = leaves.nodes.len() - 1;
        for (i, node) in leaves.nodes.iter().enumerate() {
            cbmt_node_copy(&mut tree.nodes[offset + i], node);
        }

        for i in 0..leaves.nodes.len() - 1 {
            let rev_idx = leaves.nodes.len() - 2 - i;
            let left = tree.nodes[(rev_idx << 1) + 1].clone();
            let right = tree.nodes[(rev_idx << 1) + 2].clone();
            tree.nodes[rev_idx] = merge(merge_ctx, &left, &right);
        }
    }

    match cbmt_tree_build_proof(&tree, leaf_indices) {
        Ok(result) => {
            *proof = result;
            0
        }
        Err(err) => err,
    }
}
