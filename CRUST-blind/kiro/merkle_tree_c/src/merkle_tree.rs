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
    for i in 0..width {
        let tmp = left[i];
        left[i] = right[i];
        right[i] = tmp;
    }
}

pub fn cbmt_simple_bubble_sort<T>(slice: &mut [T], cmp: fn(&T, &T) -> i32) {
    let len = slice.len();
    for i in 0..len.saturating_sub(1) {
        for j in (i + 1)..len {
            if cmp(&slice[i], &slice[j]) > 0 {
                slice.swap(i, j);
            }
        }
    }
}

pub fn cbmt_uint32_reverse_cmp(left: &u32, right: &u32) -> i32 {
    // reverse order: right - left, but careful with wrapping
    (*right as i32).wrapping_sub(*left as i32)
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
    dest.bytes = src.bytes;
}

pub fn cbmt_node_cmp(left: &CbmtNode, right: &CbmtNode) -> i32 {
    // CBMT_NODE_I32 mode: interpret as i32
    let left_value = i32::from_ne_bytes(left.bytes);
    let right_value = i32::from_ne_bytes(right.bytes);
    left_value.wrapping_sub(right_value)
}

pub fn cbmt_node_pair_reverse_cmp(left: &CbmtNodePair, right: &CbmtNodePair) -> i32 {
    // reverse order
    (right.index as i32).wrapping_sub(left.index as i32)
}

fn cbmt_is_left(index: u32) -> bool {
    (index & 1) == 1
}

fn cbmt_parent(index: u32) -> u32 {
    if index == 0 { 0 } else { (index - 1) >> 1 }
}

fn cbmt_sibling(index: u32) -> u32 {
    if index == 0 { 0 } else { ((index + 1) ^ 1) - 1 }
}

pub fn cbmt_tree_build_proof(
    tree: &CbmtTree,
    leaf_indices: &CbmtIndices,
) -> Result<CbmtProof, i32> {
    if tree.length == 0 || leaf_indices.values.is_empty() {
        return Err(CBMT_ERROR_BUILD_PROOF);
    }

    let leaves_count = (tree.length >> 1) + 1;

    // Build queue of tree-indices (leaf_index + leaves_count - 1)
    let mut queue_values: Vec<u32> = leaf_indices
        .values
        .iter()
        .map(|&v| v + (leaves_count as u32 - 1))
        .collect();

    // Sort in reverse (descending) order
    cbmt_simple_bubble_sort(&mut queue_values, cbmt_uint32_reverse_cmp);

    if queue_values[0] >= ((leaves_count as u32) << 1) - 1 {
        return Err(CBMT_ERROR_BUILD_PROOF);
    }

    let mut lemmas: Vec<CbmtNode> = Vec::new();
    let mut q = std::collections::VecDeque::from(queue_values);

    while let Some(index) = q.pop_front() {
        if index == 0 {
            if !q.is_empty() {
                return Err(CBMT_FATAL_BUILD_PROOF);
            }
            break;
        }

        let sibling = cbmt_sibling(index);
        if q.front() == Some(&sibling) {
            q.pop_front();
        } else {
            lemmas.push(tree.nodes[sibling as usize].clone());
        }

        let parent = cbmt_parent(index);
        if parent != 0 {
            q.push_back(parent);
        }
    }

    // Build sorted indices for the proof
    let mut proof_indices: Vec<u32> = leaf_indices
        .values
        .iter()
        .map(|&v| v + (leaves_count as u32 - 1))
        .collect();

    // Sort indices by the node values at those positions
    for i in 0..proof_indices.len().saturating_sub(1) {
        for j in (i + 1)..proof_indices.len() {
            let li = proof_indices[i] as usize;
            let ri = proof_indices[j] as usize;
            if cbmt_node_cmp(&tree.nodes[li], &tree.nodes[ri]) > 0 {
                proof_indices.swap(i, j);
            }
        }
    }

    Ok(CbmtProof {
        indices: CbmtIndices {
            values: proof_indices,
            capacity: leaf_indices.values.len(),
        },
        lemmas,
    })
}

pub fn cbmt_tree_root(tree: &CbmtTree) -> CbmtNode {
    if tree.length == 0 {
        CbmtNode { bytes: [0; CBMT_NODE_SIZE] }
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

    // Clone and sort leaves by node value
    let mut sorted_leaves = leaves.nodes.clone();
    cbmt_simple_bubble_sort(&mut sorted_leaves, cbmt_node_cmp);

    // Build (index, node) pairs
    let mut queue: std::collections::VecDeque<CbmtNodePair> = sorted_leaves
        .iter()
        .enumerate()
        .map(|(i, node)| CbmtNodePair {
            index: proof.indices.values[i],
            node: node.clone(),
        })
        .collect();

    // Sort pairs in reverse order by index (descending)
    let mut pairs_vec: Vec<CbmtNodePair> = queue.into_iter().collect();
    cbmt_simple_bubble_sort(&mut pairs_vec, cbmt_node_pair_reverse_cmp);
    queue = std::collections::VecDeque::from(pairs_vec);

    let mut lemmas_offset = 0usize;

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

        let sibling_idx = cbmt_sibling(index);
        let sibling = if queue.front().map(|p| p.index) == Some(sibling_idx) {
            Some(queue.pop_front().unwrap().node)
        } else if lemmas_offset < proof.lemmas.len() {
            let s = proof.lemmas[lemmas_offset].clone();
            lemmas_offset += 1;
            Some(s)
        } else {
            None
        };

        if let Some(sib) = sibling {
            let parent_node = if cbmt_is_left(index) {
                merge(merge_ctx, &node, &sib)
            } else {
                merge(merge_ctx, &sib, &node)
            };
            queue.push_back(CbmtNodePair {
                index: cbmt_parent(index),
                node: parent_node,
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
    if leaves.nodes.len() != proof.indices.values.len() || leaves.nodes.is_empty() {
        return CBMT_ERROR_PROOF_ROOT;
    }

    let mut sorted_leaves = leaves.nodes.clone();
    cbmt_simple_bubble_sort(&mut sorted_leaves, cbmt_node_cmp);

    let mut queue: std::collections::VecDeque<CbmtNodePair> = sorted_leaves
        .iter()
        .enumerate()
        .map(|(i, node)| CbmtNodePair {
            index: proof.indices.values[i],
            node: node.clone(),
        })
        .collect();

    let mut pairs_vec: Vec<CbmtNodePair> = queue.into_iter().collect();
    cbmt_simple_bubble_sort(&mut pairs_vec, cbmt_node_pair_reverse_cmp);
    queue = std::collections::VecDeque::from(pairs_vec);

    let mut lemmas_offset = 0usize;
    let mut target_root = CbmtNode::default();
    let mut found_root = false;

    while let Some(pair_current) = queue.pop_front() {
        let index = pair_current.index;
        let node = pair_current.node;

        if index == 0 {
            if proof.lemmas.len() == lemmas_offset && queue.is_empty() {
                target_root = node;
                found_root = true;
                break;
            } else {
                return CBMT_ERROR_PROOF_ROOT;
            }
        }

        let sibling_idx = cbmt_sibling(index);
        let sibling = if queue.front().map(|p| p.index) == Some(sibling_idx) {
            Some(queue.pop_front().unwrap().node)
        } else if lemmas_offset < proof.lemmas.len() {
            let s = proof.lemmas[lemmas_offset].clone();
            lemmas_offset += 1;
            Some(s)
        } else {
            None
        };

        if let Some(sib) = sibling {
            let parent_node = if cbmt_is_left(index) {
                merge(None, &node, &sib)
            } else {
                merge(None, &sib, &node)
            };
            queue.push_back(CbmtNodePair {
                index: cbmt_parent(index),
                node: parent_node,
            });
        }
    }

    if !found_root {
        return CBMT_ERROR_PROOF_ROOT;
    }

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
        return Ok(CbmtNode { bytes: [0; CBMT_NODE_SIZE] });
    }

    let mut queue: std::collections::VecDeque<CbmtNode> = std::collections::VecDeque::new();

    // Process pairs from the end
    let mut i = length as i64 - 1;
    while i > 0 {
        let left = &leaves.nodes[i as usize - 1];
        let right = &leaves.nodes[i as usize];
        let merged = merge(None, left, right);
        queue.push_back(merged);
        i -= 2;
    }
    // If odd number of leaves, push first leaf to front
    if length % 2 == 1 {
        queue.push_front(leaves.nodes[0].clone());
    }

    while queue.len() > 1 {
        let right = queue.pop_front().unwrap();
        let left = queue.pop_front().unwrap();
        let merged = merge(None, &left, &right);
        queue.push_back(merged);
    }

    Ok(queue.pop_front().unwrap())
}

pub fn cbmt_build_merkle_tree(
    tree: &mut CbmtTree,
    leaves: &CbmtLeaves,
    merge: fn(Option<&mut ()>, &CbmtNode, &CbmtNode) -> CbmtNode,
) -> i32 {
    if leaves.nodes.is_empty() {
        tree.length = 0;
        tree.nodes.clear();
        return 0;
    }

    let length = leaves.nodes.len() * 2 - 1;
    tree.length = length;
    tree.nodes.resize(length, CbmtNode::default());

    let offset = leaves.nodes.len() - 1;
    for i in 0..leaves.nodes.len() {
        tree.nodes[offset + i] = leaves.nodes[i].clone();
    }

    for i in (0..leaves.nodes.len() - 1).rev() {
        let left = tree.nodes[i * 2 + 1].clone();
        let right = tree.nodes[i * 2 + 2].clone();
        tree.nodes[i] = merge(None, &left, &right);
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
    // Build tree first using a wrapper
    let mut tree = CbmtTree::default();

    // We need to build the tree. Since merge signature differs, build manually.
    if leaves.nodes.is_empty() {
        tree.length = 0;
    } else {
        let length = leaves.nodes.len() * 2 - 1;
        tree.length = length;
        tree.nodes.resize(length, CbmtNode::default());

        let offset = leaves.nodes.len() - 1;
        for i in 0..leaves.nodes.len() {
            tree.nodes[offset + i] = leaves.nodes[i].clone();
        }
        for i in (0..leaves.nodes.len() - 1).rev() {
            let left = tree.nodes[i * 2 + 1].clone();
            let right = tree.nodes[i * 2 + 2].clone();
            tree.nodes[i] = merge(merge_ctx, &left, &right);
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
