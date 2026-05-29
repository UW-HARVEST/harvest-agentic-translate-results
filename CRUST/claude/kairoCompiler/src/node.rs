use crate::vector::{
    vector_back_ptr_or_null, vector_empty, vector_pop, vector_push, Vector,
};
use std::sync::Mutex;
use lazy_static::lazy_static;

// Re-use the same Node type from compiler
pub use crate::compiler::Node;
pub use crate::compiler::NodeBinded;

// Global list of all nodes. We store them here so we can push references (indices) into the vectors.
lazy_static! {
    pub static ref NODES: Mutex<Vec<Node>> = Mutex::new(Vec::new());
}
// Emulates `struct vector* node_vector`
lazy_static! {
    pub static ref NODE_VECTOR: Mutex<Option<Vector>> = Mutex::new(None);
}
// Emulates `struct vector* node_vector_root`
lazy_static! {
    pub static ref NODE_VECTOR_ROOT: Mutex<Option<Vector>> = Mutex::new(None);
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
    let mut nv = NODE_VECTOR.lock().unwrap();
    *nv = Some(vec);
    let mut nvr = NODE_VECTOR_ROOT.lock().unwrap();
    *nvr = Some(root_vec);
}

/// Pushes a node onto node_vector (after appending it to the global storage).
pub fn node_push(node: &Node) {
    let mut nodes = NODES.lock().unwrap();
    let idx = nodes.len() as u64;
    nodes.push(node.clone());
    drop(nodes);
    let mut nv = NODE_VECTOR.lock().unwrap();
    if let Some(v) = nv.as_mut() {
        vector_push(v, &encode_index(idx));
    }
}

/// Returns the last node or None if empty (like node_peek_or_null).
pub fn node_peek_or_null() -> Option<Node> {
    let mut nv = NODE_VECTOR.lock().unwrap();
    if let Some(v) = nv.as_mut() {
        if let Some(bytes) = vector_back_ptr_or_null(v) {
            if let Some(idx) = decode_index(bytes) {
                let nodes = NODES.lock().unwrap();
                if let Some(n) = nodes.get(idx as usize) {
                    return Some(n.clone());
                }
            }
        }
    }
    None
}

/// Returns the last Node. If none, returns default. Equivalent to node_peek in the original C code.
pub fn node_peek() -> Node {
    node_peek_or_null().unwrap_or_default()
}

/// Pops the last node. Also checks if the same index matches node_vector_root top, popping that too.
pub fn node_pop() -> Node {
    let mut nv = NODE_VECTOR.lock().unwrap();
    let mut nvr = NODE_VECTOR_ROOT.lock().unwrap();
    let last_node_idx = if let Some(v) = nv.as_mut() {
        vector_back_ptr_or_null(v).and_then(|b| decode_index(b))
    } else {
        None
    };
    let last_root_idx = if let Some(rv) = nvr.as_mut() {
        if !vector_empty(rv) {
            vector_back_ptr_or_null(rv).and_then(|b| decode_index(b))
        } else {
            None
        }
    } else {
        None
    };
    if let Some(v) = nv.as_mut() {
        vector_pop(v);
    }
    if let (Some(li), Some(lri)) = (last_node_idx, last_root_idx) {
        if li == lri {
            if let Some(rv) = nvr.as_mut() {
                vector_pop(rv);
            }
        }
    }
    if let Some(idx) = last_node_idx {
        let nodes = NODES.lock().unwrap();
        if let Some(n) = nodes.get(idx as usize) {
            return n.clone();
        }
    }
    Node::default()
}

/// Creates a new node from a template node, pushing it onto node_vector and returning the clone.
pub fn node_create(template: &Node) -> Node {
    let new_node = template.clone();
    node_push(&new_node);
    new_node
}
