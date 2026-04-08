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
/// Global list of all nodes.
lazy_static! {
    pub static ref NODES: Mutex<Vec<Node>> = Mutex::new(Vec::new());
}
/// Emulates `struct vector* node_vector`
lazy_static! {
    pub static ref NODE_VECTOR: Mutex<Option<Vector>> = Mutex::new(None);
}
/// Emulates `struct vector* node_vector_root`
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

fn get_node_by_index(idx: u64) -> Option<Node> {
    let nodes = NODES.lock().unwrap();
    nodes.get(idx as usize).cloned()
}

/// Sets the two global vectors used for node push/pop.
pub fn node_set_vector(vec: Vector, root_vec: Vector) {
    let mut nv = NODE_VECTOR.lock().unwrap();
    *nv = Some(vec);
    let mut nvr = NODE_VECTOR_ROOT.lock().unwrap();
    *nvr = Some(root_vec);
}
/// Pushes a node onto node_vector.
pub fn node_push(node: &Node) {
    let idx = {
        let mut nodes = NODES.lock().unwrap();
        let idx = nodes.len() as u64;
        nodes.push(node.clone());
        idx
    };
    let encoded = encode_index(idx);
    let mut nv = NODE_VECTOR.lock().unwrap();
    if let Some(ref mut v) = *nv {
        vector_push(v, &encoded);
    }
}
/// Returns the last node or None if empty.
pub fn node_peek_or_null() -> Option<Node> {
    let mut nv = NODE_VECTOR.lock().unwrap();
    if let Some(ref mut v) = *nv {
        let bytes = vector_back_ptr_or_null(v)?;
        let idx = decode_index(bytes)?;
        get_node_by_index(idx)
    } else {
        None
    }
}
/// Returns the last Node. If none, returns default.
pub fn node_peek() -> Node {
    let mut nv = NODE_VECTOR.lock().unwrap();
    if let Some(ref mut v) = *nv {
        if let Some(bytes) = vector_back_ptr(v) {
            if let Some(idx) = decode_index(bytes) {
                if let Some(n) = get_node_by_index(idx) {
                    return n;
                }
            }
        }
    }
    Node::default()
}
/// Pops the last node.
pub fn node_pop() -> Node {
    let last_idx = {
        let mut nv = NODE_VECTOR.lock().unwrap();
        let v = nv.as_mut().expect("node_vector not set");
        let bytes = vector_back_ptr(v).expect("empty node_vector");
        decode_index(bytes).expect("bad index")
    };
    let last_node = get_node_by_index(last_idx).unwrap_or_default();

    // Check if root matches
    let root_matches = {
        let mut nvr = NODE_VECTOR_ROOT.lock().unwrap();
        if let Some(ref mut rv) = *nvr {
            if !vector_empty(rv) {
                if let Some(bytes) = vector_back_ptr_or_null(rv) {
                    if let Some(root_idx) = decode_index(bytes) {
                        root_idx == last_idx
                    } else {
                        false
                    }
                } else {
                    false
                }
            } else {
                false
            }
        } else {
            false
        }
    };

    // Pop from node_vector
    {
        let mut nv = NODE_VECTOR.lock().unwrap();
        if let Some(ref mut v) = *nv {
            vector_pop(v);
        }
    }

    if root_matches {
        let mut nvr = NODE_VECTOR_ROOT.lock().unwrap();
        if let Some(ref mut rv) = *nvr {
            vector_pop(rv);
        }
    }

    last_node
}
/// Creates a new node from a template node, pushing it onto node_vector and returning the clone.
pub fn node_create(template: &Node) -> Node {
    let node = template.clone();
    // #warning "we should set the binded owner and binded function here"
    node_push(&node);
    node
}
