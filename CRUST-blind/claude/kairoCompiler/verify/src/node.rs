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
    let mut nodes = NODES.lock().unwrap();
    let idx = nodes.len() as u64;
    nodes.push(node.clone());
    drop(nodes);
    let mut nv = NODE_VECTOR.lock().unwrap();
    if let Some(vec) = nv.as_mut() {
        let esize = vector_element_size(vec);
        let bytes = encode_index(idx);
        if esize <= 8 {
            vector_push(vec, &bytes[..esize]);
        } else {
            // pad with zeros
            let mut padded = vec![0u8; esize];
            padded[..8].copy_from_slice(&bytes);
            vector_push(vec, &padded);
        }
    }
}

/// Returns the last node or None if empty.
pub fn node_peek_or_null() -> Option<Node> {
    let idx_opt: Option<u64> = {
        let mut nv = NODE_VECTOR.lock().unwrap();
        match nv.as_mut() {
            None => None,
            Some(vec) => {
                if vector_empty(vec) {
                    None
                } else {
                    match vector_back_ptr_or_null(vec) {
                        Some(back) => decode_index(back),
                        None => None,
                    }
                }
            }
        }
    };
    let idx = idx_opt?;
    let nodes = NODES.lock().unwrap();
    nodes.get(idx as usize).cloned()
}

/// Returns the last Node. If none, returns default.
pub fn node_peek() -> Node {
    let mut nv = NODE_VECTOR.lock().unwrap();
    let vec = match nv.as_mut() {
        Some(v) => v,
        None => return Node::default(),
    };
    let back = match vector_back_ptr(vec) {
        Some(b) => b,
        None => return Node::default(),
    };
    let idx = decode_index(back).unwrap_or(0);
    drop(nv);
    let nodes = NODES.lock().unwrap();
    nodes.get(idx as usize).cloned().unwrap_or_default()
}

/// Pops the last node. Also checks if the same index matches node_vector_root top, popping that too.
pub fn node_pop() -> Node {
    let mut nv = NODE_VECTOR.lock().unwrap();
    let vec = match nv.as_mut() {
        Some(v) => v,
        None => return Node::default(),
    };
    let last_idx = match vector_back_ptr(vec) {
        Some(b) => decode_index(b).unwrap_or(0),
        None => return Node::default(),
    };

    // Check root vec
    let mut root_lock = NODE_VECTOR_ROOT.lock().unwrap();
    let root_idx_opt = if let Some(rvec) = root_lock.as_mut() {
        if vector_empty(vec) {
            None
        } else if let Some(b) = vector_back_ptr_or_null(rvec) {
            Some(decode_index(b).unwrap_or(0))
        } else {
            None
        }
    } else {
        None
    };

    vector_pop(vec);

    if let Some(root_idx) = root_idx_opt {
        if last_idx == root_idx {
            if let Some(rvec) = root_lock.as_mut() {
                vector_pop(rvec);
            }
        }
    }

    drop(nv);
    drop(root_lock);

    let nodes = NODES.lock().unwrap();
    nodes.get(last_idx as usize).cloned().unwrap_or_default()
}

/// Creates a new node from a template node, pushing it onto node_vector and returning the clone.
pub fn node_create(template: &Node) -> Node {
    let new_node = template.clone();
    node_push(&new_node);
    new_node
}
