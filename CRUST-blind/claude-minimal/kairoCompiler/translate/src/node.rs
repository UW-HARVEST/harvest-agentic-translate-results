use crate::compiler::Pos;
use crate::vector::{
    vector_back_ptr, vector_back_ptr_or_null, vector_empty, vector_pop, vector_push, Vector,
};
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
    /// Emulates `struct vector* node_vector`
    static ref NODE_VECTOR: Mutex<Option<Vector>> = Mutex::new(None);
    /// Emulates `struct vector* node_vector_root`
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
    let mut nodes = NODES.lock().unwrap();
    let idx = nodes.len() as u64;
    nodes.push(node.clone());
    drop(nodes);

    let bytes = encode_index(idx);
    let mut nv = NODE_VECTOR.lock().unwrap();
    if let Some(v) = nv.as_mut() {
        vector_push(v, &bytes);
    }
}
/// Returns the last node or None if empty (like node_peek_or_null).
pub fn node_peek_or_null() -> Option<Node> {
    let mut nv = NODE_VECTOR.lock().unwrap();
    let v = nv.as_mut()?;
    if vector_empty(v) {
        return None;
    }
    let bytes = vector_back_ptr_or_null(v)?;
    let idx = decode_index(bytes)?;
    drop(nv);
    let nodes = NODES.lock().unwrap();
    nodes.get(idx as usize).cloned()
}
/// Returns the last Node. If none, returns default. Equivalent to node_peek in the original C code.
pub fn node_peek() -> Node {
    let mut nv = NODE_VECTOR.lock().unwrap();
    if let Some(v) = nv.as_mut() {
        if let Some(bytes) = vector_back_ptr(v) {
            if let Some(idx) = decode_index(bytes) {
                drop(nv);
                let nodes = NODES.lock().unwrap();
                if let Some(node) = nodes.get(idx as usize) {
                    return node.clone();
                }
            }
        }
    }
    Node::default()
}
/// Pops the last node. Also checks if the same index matches node_vector_root top, popping that too.
pub fn node_pop() -> Node {
    let mut nv = NODE_VECTOR.lock().unwrap();
    let v = match nv.as_mut() {
        Some(v) => v,
        None => return Node::default(),
    };
    let last_idx = match vector_back_ptr_or_null(v).and_then(|b| decode_index(b)) {
        Some(i) => i,
        None => return Node::default(),
    };

    let mut nvr = NODE_VECTOR_ROOT.lock().unwrap();
    let root_idx = if let Some(rv) = nvr.as_mut() {
        if vector_empty(rv) {
            None
        } else {
            vector_back_ptr_or_null(rv).and_then(|b| decode_index(b))
        }
    } else {
        None
    };

    vector_pop(v);
    if Some(last_idx) == root_idx {
        if let Some(rv) = nvr.as_mut() {
            vector_pop(rv);
        }
    }
    drop(nv);
    drop(nvr);
    let nodes = NODES.lock().unwrap();
    nodes
        .get(last_idx as usize)
        .cloned()
        .unwrap_or_default()
}
/// Creates a new node from a template node, pushing it onto node_vector and returning the clone.
pub fn node_create(template: &Node) -> Node {
    node_push(template);
    template.clone()
}
