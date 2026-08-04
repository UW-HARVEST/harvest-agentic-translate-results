use crate::compiler::Pos;
use crate::vector::{
    vector_back_ptr, vector_back_ptr_or_null, vector_create, vector_empty,
    vector_pop, vector_push, vector_element_size, Vector,
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
    static ref NODE_VECTOR: Mutex<Option<Vector>> = Mutex::new(None);
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

    let mut vec_guard = NODE_VECTOR.lock().unwrap();
    if let Some(vec) = vec_guard.as_mut() {
        let bytes = encode_index(idx);
        vector_push(vec, &bytes);
    }
}

/// Returns the last node or None if empty.
pub fn node_peek_or_null() -> Option<Node> {
    let mut vec_guard = NODE_VECTOR.lock().unwrap();
    let vec = vec_guard.as_mut()?;
    if vector_empty(vec) {
        return None;
    }
    let bytes = vector_back_ptr_or_null(vec)?;
    let idx = decode_index(bytes)?;
    drop(vec_guard);
    let nodes = NODES.lock().unwrap();
    nodes.get(idx as usize).cloned()
}

/// Returns the last Node. If none, returns default.
pub fn node_peek() -> Node {
    node_peek_or_null().unwrap_or_default()
}

/// Pops the last node. Also checks if the same index matches node_vector_root top.
pub fn node_pop() -> Node {
    let mut vec_guard = NODE_VECTOR.lock().unwrap();
    let mut root_guard = NODE_VECTOR_ROOT.lock().unwrap();

    let vec = match vec_guard.as_mut() {
        Some(v) => v,
        None => return Node::default(),
    };

    if vector_empty(vec) {
        return Node::default();
    }

    let last_idx_bytes = vector_back_ptr(vec).map(|s| s.to_vec());
    let last_idx = last_idx_bytes
        .as_ref()
        .and_then(|b| decode_index(b))
        .unwrap_or(0);

    let root_top_idx = root_guard.as_mut().and_then(|rv| {
        if vector_empty(rv) {
            None
        } else {
            vector_back_ptr_or_null(rv).and_then(|b| decode_index(b))
        }
    });

    vector_pop(vec);

    if let Some(rti) = root_top_idx {
        if rti == last_idx {
            if let Some(rv) = root_guard.as_mut() {
                vector_pop(rv);
            }
        }
    }

    drop(vec_guard);
    drop(root_guard);

    let nodes = NODES.lock().unwrap();
    nodes.get(last_idx as usize).cloned().unwrap_or_default()
}

/// Creates a new node from a template node, pushing it onto node_vector and returning the clone.
pub fn node_create(template: &Node) -> Node {
    let node = template.clone();
    node_push(&node);
    node
}
