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

// Helper functions matching the C macros
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
        for j in (i + 1)..length {
            if cmp(&slice[i], &slice[j]) > 0 {
                slice.swap(i, j);
            }
        }
    }
}

pub fn cbmt_uint32_reverse_cmp(left: &u32, right: &u32) -> i32 {
    // Reverse: returns right - left (using wrapping like C)
    (*right).wrapping_sub(*left) as i32
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
    if capacity * width > buffer.capacity {
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
    // For NODE_SIZE = 4, the C test treats it as int32 comparison.
    // But the function in the header uses byte-by-byte for NODE_SIZE != 4 case
    // and i32 for the I32 variant. The Rust constants use 4 (matching the I32 variant).
    let left_value = i32::from_le_bytes([
        left.bytes[0],
        left.bytes[1],
        left.bytes[2],
        left.bytes[3],
    ]);
    let right_value = i32::from_le_bytes([
        right.bytes[0],
        right.bytes[1],
        right.bytes[2],
        right.bytes[3],
    ]);
    left_value.wrapping_sub(right_value)
}

pub fn cbmt_node_pair_reverse_cmp(left: &CbmtNodePair, right: &CbmtNodePair) -> i32 {
    // reverse: right.index - left.index
    (right.index).wrapping_sub(left.index) as i32
}

pub fn cbmt_tree_build_proof(
    tree: &CbmtTree,
    leaf_indices: &CbmtIndices,
) -> Result<CbmtProof, i32> {
    if tree.length == 0 || leaf_indices.values.is_empty() {
        return Err(CBMT_ERROR_BUILD_PROOF);
    }

    let leaves_count: u32 = ((tree.length >> 1) + 1) as u32;

    // Build a queue of u32 indices using a Vec-based approach
    let mut queue: std::collections::VecDeque<u32> = std::collections::VecDeque::new();
    for v in &leaf_indices.values {
        let value = v + (leaves_count - 1);
        queue.push_back(value);
    }
    // Sort in reverse order (largest first)
    let mut sorted: Vec<u32> = queue.into_iter().collect();
    sorted.sort_by(|a, b| b.cmp(a));
    let mut queue: std::collections::VecDeque<u32> = sorted.into();

    let first_value = *queue.front().ok_or(CBMT_ERROR_BUILD_PROOF)?;
    if first_value >= ((leaves_count << 1) - 1) {
        return Err(CBMT_ERROR_BUILD_PROOF);
    }

    let mut lemmas: Vec<CbmtNode> = Vec::new();

    while !queue.is_empty() {
        let index = queue.pop_front().ok_or(CBMT_ERROR_QUEUE_EMPTY)?;
        if index == 0 {
            if !queue.is_empty() {
                return Err(CBMT_FATAL_BUILD_PROOF);
            }
            break;
        }

        let sibling = cbmt_sibling(index);
        let front = queue.front().copied();
        if front == Some(sibling) {
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

    // Build sorted indices (sort by node values at those indices)
    let mut indices_values: Vec<u32> = leaf_indices
        .values
        .iter()
        .map(|v| v + (leaves_count - 1))
        .collect();

    let len = indices_values.len();
    if len > 0 {
        for i in 0..len.saturating_sub(1) {
            for j in (i + 1)..len {
                let left_index = indices_values[i];
                let right_index = indices_values[j];
                let order = cbmt_node_cmp(
                    &tree.nodes[left_index as usize],
                    &tree.nodes[right_index as usize],
                );
                if order > 0 {
                    indices_values[i] = right_index;
                    indices_values[j] = left_index;
                }
            }
        }
    }

    let capacity = indices_values.capacity();
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
        CbmtNode {
            bytes: [0u8; CBMT_NODE_SIZE],
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

    // Clone the leaves and sort them
    let mut leaves_clone: Vec<CbmtNode> = leaves.nodes.clone();
    cbmt_simple_bubble_sort(&mut leaves_clone, cbmt_node_cmp);

    // Build the queue of (index, node) pairs
    let mut queue: std::collections::VecDeque<CbmtNodePair> = std::collections::VecDeque::new();
    for i in 0..leaves.nodes.len() {
        let pair = CbmtNodePair {
            index: proof.indices.values[i],
            node: leaves_clone[i].clone(),
        };
        queue.push_back(pair);
    }

    // Sort in reverse order by index (largest first)
    let mut sorted: Vec<CbmtNodePair> = queue.into_iter().collect();
    sorted.sort_by(|a, b| b.index.cmp(&a.index));
    let mut queue: std::collections::VecDeque<CbmtNodePair> = sorted.into();

    let mut lemmas_offset: usize = 0;

    while !queue.is_empty() {
        let pair_current = match queue.pop_front() {
            Some(p) => p,
            None => return CBMT_ERROR_QUEUE_EMPTY,
        };
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

        let sibling: Option<CbmtNode> = {
            let pair_front = queue.front();
            if let Some(pf) = pair_front {
                if pf.index == cbmt_sibling(index) {
                    let pair_sibling = queue.pop_front().unwrap();
                    Some(pair_sibling.node)
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

        if let Some(sibling) = sibling {
            let parent_node = if cbmt_is_left(index) {
                merge(merge_ctx, node, &sibling)
            } else {
                merge(merge_ctx, &sibling, node)
            };
            let pair_parent = CbmtNodePair {
                index: cbmt_parent(index),
                node: parent_node,
            };
            queue.push_back(pair_parent);
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
    let mut target_root = CbmtNode {
        bytes: [0u8; CBMT_NODE_SIZE],
    };

    // Adapt the merge function: take Option<&mut ()> and convert to a fn for cbmt_proof_root.
    // The Ctx for proof verification is `Option<&mut ()>` essentially. We need to call
    // cbmt_proof_root with appropriate merge function shape.
    // Since CbmtNodeMergeFn<Ctx> = fn(&mut Ctx, &CbmtNode, &CbmtNode) -> CbmtNode,
    // we can use Ctx = Option<&mut ()>, but the function signatures differ.
    // The proof_verify merge takes `Option<&mut ()>` (by value) not `&mut Option<&mut ()>`.
    //
    // We'll just inline the logic here using leaves.nodes and the merge function directly.

    if leaves.nodes.len() != proof.indices.values.len() || leaves.nodes.is_empty() {
        return CBMT_ERROR_PROOF_ROOT;
    }

    let mut leaves_clone: Vec<CbmtNode> = leaves.nodes.clone();
    cbmt_simple_bubble_sort(&mut leaves_clone, cbmt_node_cmp);

    let mut queue: std::collections::VecDeque<CbmtNodePair> = std::collections::VecDeque::new();
    for i in 0..leaves.nodes.len() {
        let pair = CbmtNodePair {
            index: proof.indices.values[i],
            node: leaves_clone[i].clone(),
        };
        queue.push_back(pair);
    }

    let mut sorted: Vec<CbmtNodePair> = queue.into_iter().collect();
    sorted.sort_by(|a, b| b.index.cmp(&a.index));
    let mut queue: std::collections::VecDeque<CbmtNodePair> = sorted.into();

    let mut lemmas_offset: usize = 0;
    let mut got_root = false;

    while !queue.is_empty() {
        let pair_current = match queue.pop_front() {
            Some(p) => p,
            None => return CBMT_ERROR_QUEUE_EMPTY,
        };
        let index = pair_current.index;
        let node = &pair_current.node;

        if index == 0 {
            if proof.lemmas.len() == lemmas_offset && queue.is_empty() {
                cbmt_node_copy(&mut target_root, node);
                got_root = true;
                break;
            } else {
                return CBMT_ERROR_PROOF_ROOT;
            }
        }

        let sibling: Option<CbmtNode> = {
            let pair_front = queue.front();
            if let Some(pf) = pair_front {
                if pf.index == cbmt_sibling(index) {
                    let pair_sibling = queue.pop_front().unwrap();
                    Some(pair_sibling.node)
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

        if let Some(sibling) = sibling {
            let parent_node = if cbmt_is_left(index) {
                merge(None, node, &sibling)
            } else {
                merge(None, &sibling, node)
            };
            let pair_parent = CbmtNodePair {
                index: cbmt_parent(index),
                node: parent_node,
            };
            queue.push_back(pair_parent);
        }
    }

    // Match C behavior: even if the loop exits without setting target_root,
    // compare what we have (zero-initialized) to the expected root.
    let _ = got_root;
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
        return Ok(CbmtNode {
            bytes: [0u8; CBMT_NODE_SIZE],
        });
    }

    let mut queue: std::collections::VecDeque<CbmtNode> = std::collections::VecDeque::new();

    // Loop: for (int i = length - 1; i > 0; i -= 2)
    let mut i: i64 = (length as i64) - 1;
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
        // Allocate the nodes vector
        tree.nodes = vec![CbmtNode::default(); length];
        tree.capacity = length;
        tree.length = length;

        let offset = leaves_len - 1;
        for i in 0..leaves_len {
            tree.nodes[offset + i] = leaves.nodes[i].clone();
        }

        for i in 0..leaves_len - 1 {
            let rev_idx = leaves_len - 2 - i;
            let left_idx = (rev_idx << 1) + 1;
            let right_idx = (rev_idx << 1) + 2;
            let left = tree.nodes[left_idx].clone();
            let right = tree.nodes[right_idx].clone();
            tree.nodes[rev_idx] = merge(None, &left, &right);
        }
    } else {
        tree.length = 0;
        tree.capacity = 0;
        tree.nodes = Vec::new();
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
    // Build the merkle tree. Need a fn(Option<&mut ()>...) for that, but here we have
    // CbmtNodeMergeFn<Ctx>. We do the same logic inline.
    let leaves_len = leaves.nodes.len();
    let mut tree = CbmtTree::default();

    if leaves_len > 0 {
        let length = leaves_len * 2 - 1;
        tree.nodes = vec![CbmtNode::default(); length];
        tree.capacity = length;
        tree.length = length;

        let offset = leaves_len - 1;
        for i in 0..leaves_len {
            tree.nodes[offset + i] = leaves.nodes[i].clone();
        }

        for i in 0..leaves_len - 1 {
            let rev_idx = leaves_len - 2 - i;
            let left_idx = (rev_idx << 1) + 1;
            let right_idx = (rev_idx << 1) + 2;
            let left = tree.nodes[left_idx].clone();
            let right = tree.nodes[right_idx].clone();
            tree.nodes[rev_idx] = merge(merge_ctx, &left, &right);
        }
    } else {
        tree.length = 0;
        tree.capacity = 0;
        tree.nodes = Vec::new();
    }

    match cbmt_tree_build_proof(&tree, leaf_indices) {
        Ok(p) => {
            *proof = p;
            0
        }
        Err(e) => e,
    }
}
