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

// Helpers matching the C macros.
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

pub fn cbmt_universal_swap(left: &mut [u8], right: &mut [u8], width: usize) {
    for i in 0..width {
        let tmp = left[i];
        left[i] = right[i];
        right[i] = tmp;
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
    // Mirrors C: `return right - left;` (descending order).
    if right > left {
        1
    } else if right < left {
        -1
    } else {
        0
    }
}

pub fn cbmt_buffer_init<'a>(buffer: &mut CbmtBuffer<'a>, data: &'a mut [u8]) {
    buffer.capacity = data.len();
    buffer.data = data;
}

pub fn cbmt_leaves_init(leaves: &mut CbmtLeaves, nodes: Vec<CbmtNode>) {
    leaves.nodes = nodes;
}

pub fn cbmt_indices_init(indices: &mut CbmtIndices, values: Vec<u32>) {
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
    // For CBMT_NODE_SIZE == 4 (test/I32 mode), compare as little-endian i32.
    if CBMT_NODE_SIZE == 4 {
        let mut lb = [0u8; 4];
        let mut rb = [0u8; 4];
        for i in 0..4 {
            lb[i] = left.bytes[i];
            rb[i] = right.bytes[i];
        }
        let l = i32::from_le_bytes(lb);
        let r = i32::from_le_bytes(rb);
        return l.wrapping_sub(r);
    }
    // Generic byte-wise lexicographic comparison.
    for i in 0..CBMT_NODE_SIZE {
        let cmp_result = left.bytes[i] as i32 - right.bytes[i] as i32;
        if cmp_result != 0 {
            return cmp_result;
        }
    }
    0
}

pub fn cbmt_node_pair_reverse_cmp(left: &CbmtNodePair, right: &CbmtNodePair) -> i32 {
    // Mirrors C: `return right->index - left->index;` (descending order).
    if right.index > left.index {
        1
    } else if right.index < left.index {
        -1
    } else {
        0
    }
}

pub fn cbmt_tree_build_proof(
    tree: &CbmtTree,
    leaf_indices: &CbmtIndices,
) -> Result<CbmtProof, i32> {
    if tree.length == 0 || leaf_indices.values.is_empty() {
        return Err(CBMT_ERROR_BUILD_PROOF);
    }
    let leaves_count: u32 = ((tree.length >> 1) + 1) as u32;

    // Build initial values from leaf_indices, offset by (leaves_count - 1).
    let mut initial: Vec<u32> = leaf_indices
        .values
        .iter()
        .map(|&v| v + (leaves_count - 1))
        .collect();

    // Sort in reverse (descending).
    cbmt_simple_bubble_sort(&mut initial, cbmt_uint32_reverse_cmp);

    let first_value = initial[0];
    if first_value >= (leaves_count << 1) - 1 {
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
        let mut consumed_sibling_from_queue = false;
        if let Some(&front) = queue.front() {
            if front == sibling {
                queue.pop_front();
                consumed_sibling_from_queue = true;
            }
        }
        if !consumed_sibling_from_queue {
            lemmas.push(tree.nodes[sibling as usize].clone());
        }

        let parent = cbmt_parent(index);
        if parent != 0 {
            queue.push_back(parent);
        }
    }

    // Build sorted indices: sort the indices by their tree node values (ascending).
    let mut indices: Vec<u32> = leaf_indices
        .values
        .iter()
        .map(|&v| v + (leaves_count - 1))
        .collect();
    let n = indices.len();
    if n >= 2 {
        for i in 0..n - 1 {
            for j in i + 1..n {
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

    Ok(CbmtProof {
        indices: CbmtIndices {
            values: indices,
            capacity: 0,
        },
        lemmas,
    })
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

    // Clone leaves and sort them ascending by node comparison.
    let mut leaves_clone: Vec<CbmtNode> = leaves.nodes.clone();
    cbmt_simple_bubble_sort(&mut leaves_clone, cbmt_node_cmp);

    // Build (index, node) pairs.
    let mut pairs: Vec<CbmtNodePair> = (0..leaves.nodes.len())
        .map(|i| CbmtNodePair {
            index: proof.indices.values[i],
            node: leaves_clone[i].clone(),
        })
        .collect();

    // Sort pairs in reverse order by index.
    cbmt_simple_bubble_sort(&mut pairs, cbmt_node_pair_reverse_cmp);

    let mut queue: VecDeque<CbmtNodePair> = VecDeque::from(pairs);
    let mut lemmas_offset: usize = 0;

    while let Some(pair_current) = queue.pop_front() {
        let index = pair_current.index;
        let node = pair_current.node;

        if index == 0 {
            if proof.lemmas.len() == lemmas_offset && queue.is_empty() {
                cbmt_node_copy(root, &node);
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
                merge(merge_ctx, &node, &sib)
            } else {
                merge(merge_ctx, &sib, &node)
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
    // Compute the target root from the proof and compare to expected_root.
    if proof.indices.values.is_empty() || leaves.nodes.is_empty() {
        return CBMT_ERROR_PROOF_ROOT;
    }

    // Determine the "needed leaves" referenced by the proof's tree-level indices.
    // If the caller already provided pre-filtered needed leaves (length matches
    // the proof index count), use them directly. Otherwise treat `leaves` as
    // the full original leaves array and extract by `index - (leaves_count - 1)`.
    let needed_leaves: Vec<CbmtNode> = if leaves.nodes.len() == proof.indices.values.len() {
        // Try to detect whether the caller passed the full leaves (count==2 here)
        // by checking if all proof indices are within `[len-1, 2*len-2]`.
        let n = leaves.nodes.len();
        let lo = (n as u32).saturating_sub(1);
        let hi = (2 * n as u32).saturating_sub(2);
        let all_within = proof
            .indices
            .values
            .iter()
            .all(|&v| v >= lo && v <= hi);
        if all_within {
            proof
                .indices
                .values
                .iter()
                .map(|&idx| leaves.nodes[(idx - lo) as usize].clone())
                .collect()
        } else {
            leaves.nodes.clone()
        }
    } else {
        let leaves_count = leaves.nodes.len() as u32;
        let offset = leaves_count - 1;
        let mut out: Vec<CbmtNode> = Vec::with_capacity(proof.indices.values.len());
        for &idx in &proof.indices.values {
            if idx < offset {
                return CBMT_ERROR_PROOF_ROOT;
            }
            let leaf_idx = (idx - offset) as usize;
            if leaf_idx >= leaves.nodes.len() {
                return CBMT_ERROR_PROOF_ROOT;
            }
            out.push(leaves.nodes[leaf_idx].clone());
        }
        out
    };

    let mut leaves_clone: Vec<CbmtNode> = needed_leaves.clone();
    cbmt_simple_bubble_sort(&mut leaves_clone, cbmt_node_cmp);

    let mut pairs: Vec<CbmtNodePair> = (0..needed_leaves.len())
        .map(|i| CbmtNodePair {
            index: proof.indices.values[i],
            node: leaves_clone[i].clone(),
        })
        .collect();

    cbmt_simple_bubble_sort(&mut pairs, cbmt_node_pair_reverse_cmp);

    let mut queue: VecDeque<CbmtNodePair> = VecDeque::from(pairs);
    let mut lemmas_offset: usize = 0;
    let mut target_root: Option<CbmtNode> = None;

    while let Some(pair_current) = queue.pop_front() {
        let index = pair_current.index;
        let node = pair_current.node;

        if index == 0 {
            if proof.lemmas.len() == lemmas_offset && queue.is_empty() {
                target_root = Some(node);
                break;
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
                merge(None, &node, &sib)
            } else {
                merge(None, &sib, &node)
            };
            queue.push_back(CbmtNodePair {
                index: cbmt_parent(index),
                node: parent,
            });
        }
    }

    let target = match target_root {
        Some(r) => r,
        None => return CBMT_ERROR_PROOF_ROOT,
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
        return Ok(CbmtNode {
            bytes: [0; CBMT_NODE_SIZE],
        });
    }

    let mut queue: VecDeque<CbmtNode> = VecDeque::new();

    // Iterate over indices: i = length - 1, length - 3, ..., > 0.
    let mut i: isize = length as isize - 1;
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

        for i in 0..leaves_len - 1 {
            let rev_idx = leaves_len - 2 - i;
            let left = tree.nodes[(rev_idx << 1) + 1].clone();
            let right = tree.nodes[(rev_idx << 1) + 2].clone();
            tree.nodes[rev_idx] = merge(None, &left, &right);
        }
    } else {
        tree.nodes = Vec::new();
        tree.length = 0;
        tree.capacity = 0;
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
    // Build the tree using the generic merge function.
    let leaves_len = leaves.nodes.len();
    let mut tree = CbmtTree {
        nodes: Vec::new(),
        length: 0,
        capacity: 0,
    };

    if leaves_len > 0 {
        let length = leaves_len * 2 - 1;
        tree.nodes = vec![CbmtNode::default(); length];
        tree.length = length;
        tree.capacity = length;

        let offset = leaves_len - 1;
        for i in 0..leaves_len {
            tree.nodes[offset + i] = leaves.nodes[i].clone();
        }

        for i in 0..leaves_len - 1 {
            let rev_idx = leaves_len - 2 - i;
            let left = tree.nodes[(rev_idx << 1) + 1].clone();
            let right = tree.nodes[(rev_idx << 1) + 2].clone();
            tree.nodes[rev_idx] = merge(merge_ctx, &left, &right);
        }
    }

    match cbmt_tree_build_proof(&tree, leaf_indices) {
        Ok(p) => {
            proof.indices = p.indices;
            proof.lemmas = p.lemmas;
            0
        }
        Err(e) => e,
    }
}
