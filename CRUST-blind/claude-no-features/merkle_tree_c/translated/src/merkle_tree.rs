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
    // C semantics: return right - left in uint32_t arithmetic, then implicitly cast to int.
    right.wrapping_sub(*left) as i32
}

pub fn cbmt_buffer_init<'a>(buffer: &mut CbmtBuffer<'a>, data: &'a mut [u8]) {
    let capacity = data.len();
    buffer.data = data;
    buffer.capacity = capacity;
}

pub fn cbmt_leaves_init(leaves: &mut CbmtLeaves, nodes: Vec<CbmtNode>) {
    leaves.nodes = nodes;
}

pub fn cbmt_indices_init(indices: &mut CbmtIndices, values: Vec<u32>) {
    let len = values.len();
    indices.values = values;
    indices.capacity = len;
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
    if queue.length >= queue.capacity {
        return CBMT_ERROR_OVER_CAPACITY;
    }
    let offset = queue.head * queue.width;
    queue.buffer.data[offset..offset + queue.width].copy_from_slice(&item[..queue.width]);
    queue.head = (queue.head + 1) % queue.capacity;
    queue.length += 1;
    0
}

pub fn cbmt_queue_push_front(queue: &mut CbmtQueue, item: &[u8]) -> i32 {
    if queue.length >= queue.capacity {
        return CBMT_ERROR_OVER_CAPACITY;
    }
    queue.tail = (queue.tail + queue.capacity - 1) % queue.capacity;
    let offset = queue.tail * queue.width;
    queue.buffer.data[offset..offset + queue.width].copy_from_slice(&item[..queue.width]);
    queue.length += 1;
    0
}

pub fn cbmt_queue_pop_front(queue: &mut CbmtQueue, item: &mut [u8]) -> i32 {
    if queue.length == 0 {
        return CBMT_ERROR_QUEUE_EMPTY;
    }
    let offset = queue.tail * queue.width;
    item[..queue.width].copy_from_slice(&queue.buffer.data[offset..offset + queue.width]);
    queue.tail = (queue.tail + 1) % queue.capacity;
    queue.length -= 1;
    0
}

pub fn cbmt_queue_front<'a>(queue: &'a CbmtQueue<'a>) -> Option<&'a [u8]> {
    if queue.length == 0 {
        return None;
    }
    let offset = queue.tail * queue.width;
    Some(&queue.buffer.data[offset..offset + queue.width])
}

pub fn cbmt_node_copy(dest: &mut CbmtNode, src: &CbmtNode) {
    dest.bytes.copy_from_slice(&src.bytes);
}

pub fn cbmt_node_cmp(left: &CbmtNode, right: &CbmtNode) -> i32 {
    if CBMT_NODE_SIZE == 4 {
        let mut l_arr = [0u8; 4];
        let mut r_arr = [0u8; 4];
        for i in 0..4 {
            l_arr[i] = left.bytes[i];
            r_arr[i] = right.bytes[i];
        }
        let l = i32::from_le_bytes(l_arr);
        let r = i32::from_le_bytes(r_arr);
        l.wrapping_sub(r)
    } else {
        for i in 0..CBMT_NODE_SIZE {
            let cmp = left.bytes[i] as i32 - right.bytes[i] as i32;
            if cmp != 0 {
                return cmp;
            }
        }
        0
    }
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

    // Build initial queue: leaf_indices values offset to tree positions, sorted reverse.
    let mut initial: Vec<u32> = leaf_indices
        .values
        .iter()
        .map(|v| v + (leaves_count - 1))
        .collect();
    cbmt_simple_bubble_sort(&mut initial, cbmt_uint32_reverse_cmp);

    let first_value = *initial.first().ok_or(CBMT_ERROR_BUILD_PROOF)?;
    if first_value >= (leaves_count << 1) - 1 {
        return Err(CBMT_ERROR_BUILD_PROOF);
    }

    let mut queue: VecDeque<u32> = initial.into();
    let mut lemmas: Vec<CbmtNode> = Vec::new();

    while let Some(index) = queue.pop_front() {
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
            let src = &tree.nodes[sibling as usize];
            lemmas.push(src.clone());
        }

        let parent = cbmt_parent(index);
        if parent != 0 {
            queue.push_back(parent);
        }
    }

    // Build sorted indices for the proof.
    let mut indices_values: Vec<u32> = leaf_indices
        .values
        .iter()
        .map(|v| v + (leaves_count - 1))
        .collect();
    let n = indices_values.len();
    if n > 1 {
        for i in 0..n - 1 {
            for j in (i + 1)..n {
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

// Internal helper: compute the proof's root using a closure-based merge function.
// Returns Ok(Some(root)) when the algorithm completes by processing index 0.
// Returns Ok(None) when the loop exhausts the queue without reaching index 0
// (matching the C behavior of returning 0 without writing root).
fn proof_root_impl<F>(
    proof: &CbmtProof,
    leaves: &CbmtLeaves,
    mut merge: F,
) -> Result<Option<CbmtNode>, i32>
where
    F: FnMut(&CbmtNode, &CbmtNode) -> CbmtNode,
{
    if leaves.nodes.len() != proof.indices.values.len() || leaves.nodes.is_empty() {
        return Err(CBMT_ERROR_PROOF_ROOT);
    }

    // Clone and sort leaves by node_cmp (matches sorting in C).
    let mut leaves_clone: Vec<CbmtNode> = leaves.nodes.clone();
    cbmt_simple_bubble_sort(&mut leaves_clone, cbmt_node_cmp);

    // Build pairs vector, then sort by reverse index.
    let mut pairs: Vec<CbmtNodePair> = Vec::with_capacity(leaves.nodes.len());
    for i in 0..leaves.nodes.len() {
        pairs.push(CbmtNodePair {
            index: proof.indices.values[i],
            node: leaves_clone[i].clone(),
        });
    }
    cbmt_simple_bubble_sort(&mut pairs, cbmt_node_pair_reverse_cmp);

    let mut queue: VecDeque<CbmtNodePair> = pairs.into();
    let mut lemmas_offset: usize = 0;

    while let Some(pair_current) = queue.pop_front() {
        let index = pair_current.index;
        let node = pair_current.node;

        if index == 0 {
            if proof.lemmas.len() == lemmas_offset && queue.is_empty() {
                return Ok(Some(node));
            } else {
                return Err(CBMT_ERROR_PROOF_ROOT);
            }
        }

        let sibling_idx = cbmt_sibling(index);
        let sibling: Option<CbmtNode> = if let Some(pf) = queue.front() {
            if pf.index == sibling_idx {
                Some(queue.pop_front().unwrap().node)
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
        };

        if let Some(sib) = sibling {
            let parent_node = if cbmt_is_left(index) {
                merge(&node, &sib)
            } else {
                merge(&sib, &node)
            };
            let parent_idx = cbmt_parent(index);
            queue.push_back(CbmtNodePair {
                index: parent_idx,
                node: parent_node,
            });
        }
    }

    Ok(None)
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
    match proof_root_impl(proof, leaves, |l, r| merge(merge_ctx, l, r)) {
        Ok(Some(r)) => {
            *root = r;
            0
        }
        Ok(None) => 0,
        Err(e) => e,
    }
}

pub fn cbmt_proof_verify(
    proof: &CbmtProof,
    expected_root: &CbmtNode,
    leaves: &CbmtLeaves,
    merge: fn(Option<&mut ()>, &CbmtNode, &CbmtNode) -> CbmtNode,
) -> i32 {
    match proof_root_impl(proof, leaves, |l, r| merge(None, l, r)) {
        Ok(Some(target_root)) => {
            if target_root.bytes != expected_root.bytes {
                CBMT_ERROR_VERIFY_FAILED
            } else {
                0
            }
        }
        Ok(None) => CBMT_ERROR_VERIFY_FAILED,
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
            bytes: [0; CBMT_NODE_SIZE],
        });
    }

    let capacity = (length + 1) >> 1;
    let mut queue: VecDeque<CbmtNode> = VecDeque::with_capacity(capacity);

    if length >= 2 {
        let mut i = length - 1;
        loop {
            let merged = merge(None, &leaves.nodes[i - 1], &leaves.nodes[i]);
            queue.push_back(merged);
            if i < 2 {
                break;
            }
            i -= 2;
            if i == 0 {
                break;
            }
        }
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

// Internal helper: build a merkle tree using a closure-based merge function.
fn build_tree_impl<F>(tree: &mut CbmtTree, leaves: &CbmtLeaves, mut merge: F)
where
    F: FnMut(&CbmtNode, &CbmtNode) -> CbmtNode,
{
    let leaves_len = leaves.nodes.len();
    if leaves_len > 0 {
        let length = leaves_len * 2 - 1;
        tree.nodes = vec![
            CbmtNode {
                bytes: [0; CBMT_NODE_SIZE]
            };
            length
        ];
        tree.length = length;
        tree.capacity = length;

        let offset = leaves_len - 1;
        for i in 0..leaves_len {
            tree.nodes[offset + i] = leaves.nodes[i].clone();
        }

        if leaves_len > 1 {
            for i in 0..leaves_len - 1 {
                let rev_idx = leaves_len - 2 - i;
                let left_idx = (rev_idx << 1) + 1;
                let right_idx = (rev_idx << 1) + 2;
                let merged = {
                    let left = &tree.nodes[left_idx];
                    let right = &tree.nodes[right_idx];
                    merge(left, right)
                };
                tree.nodes[rev_idx] = merged;
            }
        }
    } else {
        tree.length = 0;
        tree.nodes = Vec::new();
        tree.capacity = 0;
    }
}

pub fn cbmt_build_merkle_tree(
    tree: &mut CbmtTree,
    leaves: &CbmtLeaves,
    merge: fn(Option<&mut ()>, &CbmtNode, &CbmtNode) -> CbmtNode,
) -> i32 {
    build_tree_impl(tree, leaves, |l, r| merge(None, l, r));
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
    let mut tree = CbmtTree::default();
    build_tree_impl(&mut tree, leaves, |l, r| merge(merge_ctx, l, r));
    match cbmt_tree_build_proof(&tree, leaf_indices) {
        Ok(p) => {
            *proof = p;
            0
        }
        Err(e) => e,
    }
}
