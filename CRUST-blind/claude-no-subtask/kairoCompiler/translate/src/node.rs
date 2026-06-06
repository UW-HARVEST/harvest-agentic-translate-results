use crate::compiler::Pos;
use crate::vector::Vector;
use std::sync::Mutex;
use lazy_static::lazy_static;

#[derive(Debug, Default, Clone)]
pub struct NodeBinded {
    pub owner: Option<Box<Node>>,
    pub function: Option<Box<Node>>,
}

#[derive(Debug, Default, Clone)]
pub struct Node {
    pub r#type: i32,
    pub flags: i32,
    pub pos: Pos,
    pub binded: NodeBinded,
    pub cval: Option<char>,
    pub sval: Option<String>,
    pub inum: Option<u32>,
    pub lnum: Option<u64>,
    pub llnum: Option<u64>,
}

// Global list of all nodes. We store them here so we can push references (indices) into the vectors.
lazy_static! {
    static ref NODES: Mutex<Vec<Node>> = Mutex::new(Vec::new());
}

// Emulates `struct vector* node_vector`
lazy_static! {
    static ref NODE_VECTOR: Mutex<Option<Vector>> = Mutex::new(None);
}

// Emulates `struct vector* node_vector_root`
lazy_static! {
    static ref NODE_VECTOR_ROOT: Mutex<Option<Vector>> = Mutex::new(None);
}

/// Converts a u64 index to 8 bytes LE.
fn encode_index(idx: u64) -> [u8; 8] {
    idx.to_le_bytes()
}

/// Converts 8 bytes LE to u64.
fn decode_index(bytes: &[u8]) -> Option<u64> {
    if bytes.len() < 8 {
        return None;
    }
    let mut arr = [0u8; 8];
    arr.copy_from_slice(&bytes[..8]);
    Some(u64::from_le_bytes(arr))
}

/// Sets the two global vectors used for node push/pop.
pub fn node_set_vector(vec: Vector, root_vec: Vector) {
    *NODE_VECTOR.lock().unwrap() = Some(vec);
    *NODE_VECTOR_ROOT.lock().unwrap() = Some(root_vec);
}

/// Pushes a node onto node_vector.
pub fn node_push(node: &Node) {
    // Add the node to the global NODES vector and store its index
    let mut nodes = NODES.lock().unwrap();
    let idx = nodes.len() as u64;
    nodes.push(node.clone());
    drop(nodes);

    let mut node_vector = NODE_VECTOR.lock().unwrap();
    if let Some(ref mut v) = *node_vector {
        let bytes = encode_index(idx);
        crate::vector::vector_push(v, &bytes);
    }
}

/// Returns the last node or None if empty (like node_peek_or_null).
pub fn node_peek_or_null() -> Option<Node> {
    let mut node_vector = NODE_VECTOR.lock().unwrap();
    let v = node_vector.as_mut()?;
    let bytes = crate::vector::vector_back_or_null(v)?;
    let idx = decode_index(bytes)?;
    let nodes = NODES.lock().unwrap();
    nodes.get(idx as usize).cloned()
}

/// Returns the last Node. If none, returns default. Equivalent to node_peek in the original C code.
pub fn node_peek() -> Node {
    let mut node_vector = NODE_VECTOR.lock().unwrap();
    let v = match node_vector.as_mut() {
        Some(v) => v,
        None => return Node::default(),
    };
    let bytes_opt = crate::vector::vector_back(v);
    let idx = match bytes_opt {
        Some(b) => match decode_index(b) {
            Some(i) => i,
            None => return Node::default(),
        },
        None => return Node::default(),
    };
    let nodes = NODES.lock().unwrap();
    nodes.get(idx as usize).cloned().unwrap_or_default()
}

/// Pops the last node. Also checks if the same index matches node_vector_root top, popping that too.
pub fn node_pop() -> Node {
    let last_idx;
    let last_root_idx;
    {
        let mut node_vector = NODE_VECTOR.lock().unwrap();
        let v = match node_vector.as_mut() {
            Some(v) => v,
            None => return Node::default(),
        };
        last_idx = crate::vector::vector_back_ptr(v)
            .and_then(|b| decode_index(b));
    }
    {
        let mut node_vector_root = NODE_VECTOR_ROOT.lock().unwrap();
        let mut node_vector = NODE_VECTOR.lock().unwrap();
        let nv = node_vector.as_mut();
        last_root_idx = if let Some(v) = nv {
            if crate::vector::vector_empty(v) {
                None
            } else {
                node_vector_root.as_mut()
                    .and_then(|rv| crate::vector::vector_back_ptr(rv))
                    .and_then(|b| decode_index(b))
            }
        } else {
            None
        };
    }
    {
        let mut node_vector = NODE_VECTOR.lock().unwrap();
        if let Some(v) = node_vector.as_mut() {
            if !crate::vector::vector_empty(v) {
                crate::vector::vector_pop(v);
            }
        }
    }
    if last_idx.is_some() && last_idx == last_root_idx {
        let mut node_vector_root = NODE_VECTOR_ROOT.lock().unwrap();
        if let Some(rv) = node_vector_root.as_mut() {
            if !crate::vector::vector_empty(rv) {
                crate::vector::vector_pop(rv);
            }
        }
    }

    if let Some(idx) = last_idx {
        let nodes = NODES.lock().unwrap();
        nodes.get(idx as usize).cloned().unwrap_or_default()
    } else {
        Node::default()
    }
}

/// Creates a new node from a template node, pushing it onto node_vector and returning the clone.
pub fn node_create(template: &Node) -> Node {
    let node = template.clone();
    node_push(&node);
    node
}

/// Internal helper used by the parser: peeks the latest index from NODE_VECTOR
/// and pushes it onto NODE_VECTOR_ROOT.
pub fn __internal_push_root_with_back() {
    let mut nv = NODE_VECTOR.lock().unwrap();
    let mut root = NODE_VECTOR_ROOT.lock().unwrap();
    if let (Some(v), Some(r)) = (nv.as_mut(), root.as_mut()) {
        if let Some(bytes) = crate::vector::vector_back_or_null(v) {
            let bytes_owned = bytes.to_vec();
            crate::vector::vector_push(r, &bytes_owned);
        }
    }
}

/// Returns the current node_vec out of the global slot (taking ownership).
pub fn __internal_take_node_vec() -> Option<Vector> {
    NODE_VECTOR.lock().unwrap().take()
}

/// Returns the current node_tree_vec out of the global slot (taking ownership).
pub fn __internal_take_node_tree_vec() -> Option<Vector> {
    NODE_VECTOR_ROOT.lock().unwrap().take()
}
