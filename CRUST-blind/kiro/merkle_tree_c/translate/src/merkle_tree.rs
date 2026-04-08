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

fn cbmt_is_left(index: u32) -> bool {
    (index & 1) == 1
}

fn cbmt_parent(index: u32) -> u32 {
    if index == 0 { 0 } else { (index - 1) >> 1 }
}

fn cbmt_sibling(index: u32) -> u32 {
    if index == 0 { 0 } else { ((index + 1) ^ 1) - 1 }
}

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
    dest.bytes.copy_from_slice(&src.bytes);
}

pub fn cbmt_node_cmp(left: &CbmtNode, right: &CbmtNode) -> i32 {
    // CBMT_NODE_I32 mode: interpret as i32
    let lv = i32::from_le_bytes(left.bytes);
    let rv = i32::from_le_bytes(right.bytes);
    lv.wrapping_sub(rv)
}

pub fn cbmt_node_pair_reverse_cmp(left: &CbmtNodePair, right: &CbmtNodePair) -> i32 {
    (right.index as i32).wrapping_sub(left.index as i32)
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
    let mut queue: Vec<u32> = leaf_indices
        .values
        .iter()
        .map(|&v| v + (leaves_count as u32 - 1))
        .collect();

    // Sort in reverse (descending) order
    cbmt_simple_bubble_sort(&mut queue, cbmt_uint32_reverse_cmp);

    if *queue.first().unwrap() >= ((leaves_count as u32) << 1) - 1 {
        return Err(CBMT_ERROR_BUILD_PROOF);
    }

    let mut lemmas: Vec<CbmtNode> = Vec::new();

    while !queue.is_empty() {
        let index = queue.remove(0);
        if index == 0 {
            if !queue.is_empty() {
                return Err(CBMT_FATAL_BUILD_PROOF);
            }
            break;
        }

        let sibling = cbmt_sibling(index);
        if !queue.is_empty() && queue[0] == sibling {
            queue.remove(0);
        } else {
            lemmas.push(tree.nodes[sibling as usize].clone());
        }

        let parent = cbmt_parent(index);
        if parent != 0 {
            queue.push(parent);
        }
    }

    // Build sorted indices: leaf_index + leaves_count - 1, sorted by node value
    let mut index_node_pairs: Vec<(u32, CbmtNode)> = leaf_indices
        .values
        .iter()
        .map(|&v| {
            let tree_idx = v + (leaves_count as u32 - 1);
            (tree_idx, tree.nodes[tree_idx as usize].clone())
        })
        .collect();

    // Bubble sort by node comparison
    for i in 0..index_node_pairs.len().saturating_sub(1) {
        for j in (i + 1)..index_node_pairs.len() {
            if cbmt_node_cmp(&index_node_pairs[i].1, &index_node_pairs[j].1) > 0 {
                index_node_pairs.swap(i, j);
            }
        }
    }

    let sorted_indices: Vec<u32> = index_node_pairs.iter().map(|(idx, _)| *idx).collect();

    Ok(CbmtProof {
        indices: CbmtIndices {
            values: sorted_indices,
            capacity: leaf_indices.values.len(),
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

    // Build queue of (index, node) pairs
    let mut queue: Vec<CbmtNodePair> = Vec::new();
    for i in 0..sorted_leaves.len() {
        queue.push(CbmtNodePair {
            index: proof.indices.values[i],
            node: sorted_leaves[i].clone(),
        });
    }
    cbmt_simple_bubble_sort(&mut queue, cbmt_node_pair_reverse_cmp);

    let mut lemmas_offset = 0usize;

    while !queue.is_empty() {
        let pair_current = queue.remove(0);
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

        let sibling_node;
        if !queue.is_empty() && queue[0].index == cbmt_sibling(index) {
            let pair_sibling = queue.remove(0);
            sibling_node = pair_sibling.node;
        } else if lemmas_offset < proof.lemmas.len() {
            sibling_node = proof.lemmas[lemmas_offset].clone();
            lemmas_offset += 1;
        } else {
            continue;
        }

        let parent_node = if cbmt_is_left(index) {
            merge(merge_ctx, &node, &sibling_node)
        } else {
            merge(merge_ctx, &sibling_node, &node)
        };

        queue.push(CbmtNodePair {
            index: cbmt_parent(index),
            node: parent_node,
        });
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

    let mut queue: Vec<CbmtNodePair> = Vec::new();
    for i in 0..sorted_leaves.len() {
        queue.push(CbmtNodePair {
            index: proof.indices.values[i],
            node: sorted_leaves[i].clone(),
        });
    }
    cbmt_simple_bubble_sort(&mut queue, cbmt_node_pair_reverse_cmp);

    let mut lemmas_offset = 0usize;
    let mut target_root = CbmtNode::default();

    while !queue.is_empty() {
        let pair_current = queue.remove(0);
        let index = pair_current.index;
        let node = pair_current.node;

        if index == 0 {
            if proof.lemmas.len() == lemmas_offset && queue.is_empty() {
                target_root = node;
                if target_root.bytes != expected_root.bytes {
                    return CBMT_ERROR_VERIFY_FAILED;
                }
                return 0;
            } else {
                return CBMT_ERROR_PROOF_ROOT;
            }
        }

        let sibling_node;
        if !queue.is_empty() && queue[0].index == cbmt_sibling(index) {
            let pair_sibling = queue.remove(0);
            sibling_node = pair_sibling.node;
        } else if lemmas_offset < proof.lemmas.len() {
            sibling_node = proof.lemmas[lemmas_offset].clone();
            lemmas_offset += 1;
        } else {
            continue;
        }

        let parent_node = if cbmt_is_left(index) {
            merge(None, &node, &sibling_node)
        } else {
            merge(None, &sibling_node, &node)
        };

        queue.push(CbmtNodePair {
            index: cbmt_parent(index),
            node: parent_node,
        });
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

    let mut queue: Vec<CbmtNode> = Vec::new();

    // Process pairs from end
    let mut i = length as i64 - 1;
    while i > 0 {
        let left = &leaves.nodes[(i - 1) as usize];
        let right = &leaves.nodes[i as usize];
        let merged = merge(None, left, right);
        queue.push(merged);
        i -= 2;
    }

    // If odd number of leaves, prepend the first leaf
    if length % 2 == 1 {
        queue.insert(0, leaves.nodes[0].clone());
    }

    while queue.len() > 1 {
        // Pop front two (right first, then left, matching C code)
        let right = queue.remove(0);
        let left = queue.remove(0);
        let merged = merge(None, &left, &right);
        queue.push(merged);
    }

    Ok(queue.remove(0))
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
    // Build tree first using an adapter
    let mut tree = CbmtTree::default();

    // We need to build the tree using the generic merge fn
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
