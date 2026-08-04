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
    let mut nv = NODE_VECTOR.lock().unwrap();
    *nv = Some(vec);
    let mut nvr = NODE_VECTOR_ROOT.lock().unwrap();
    *nvr = Some(root_vec);
    // Reset the underlying nodes pool so each test starts fresh.
    let mut nodes = NODES.lock().unwrap();
    nodes.clear();
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

/// Returns the last node or None if empty.
pub fn node_peek_or_null() -> Option<Node> {
    let mut nv = NODE_VECTOR.lock().unwrap();
    let v = nv.as_mut()?;
    let bytes = vector_back_ptr_or_null(v)?;
    let idx = decode_index(bytes)?;
    let nodes = NODES.lock().unwrap();
    nodes.get(idx as usize).cloned()
}

/// Returns the last Node.
pub fn node_peek() -> Node {
    let mut nv = NODE_VECTOR.lock().unwrap();
    let v = nv.as_mut().expect("node_vector not set");
    let bytes = vector_back_ptr(v).expect("node_vector empty");
    let idx = decode_index(bytes).expect("decode failure");
    let nodes = NODES.lock().unwrap();
    nodes[idx as usize].clone()
}

/// Pops the last node. Also checks if the same index matches node_vector_root top.
pub fn node_pop() -> Node {
    let mut nv = NODE_VECTOR.lock().unwrap();
    let v = nv.as_mut().expect("node_vector not set");
    let bytes = vector_back_ptr(v).expect("node_vector empty");
    let last_idx = decode_index(bytes).expect("decode failure");

    // Compute "last_node_root" as the C does: vector_empty(node_vector) ? NULL : back_ptr(root)
    // Since we just confirmed node_vector is non-empty (vector_back_ptr succeeded), check root.
    let mut nvr = NODE_VECTOR_ROOT.lock().unwrap();
    let last_root_idx_opt: Option<u64> = if let Some(rv) = nvr.as_mut() {
        if vector_empty(rv) {
            None
        } else {
            vector_back_ptr(rv).and_then(|b| decode_index(b))
        }
    } else {
        None
    };

    vector_pop(v);

    if let Some(root_idx) = last_root_idx_opt {
        if root_idx == last_idx {
            if let Some(rv) = nvr.as_mut() {
                vector_pop(rv);
            }
        }
    }

    let nodes = NODES.lock().unwrap();
    nodes[last_idx as usize].clone()
}

/// Creates a new node from a template node, pushing it onto node_vector and returning the clone.
pub fn node_create(template: &Node) -> Node {
    let n = template.clone();
    node_push(&n);
    n
}
