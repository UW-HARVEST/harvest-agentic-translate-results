use crate::compiler::{Pos, Node, NodeBinded};
use crate::vector::{
    vector_back_ptr, vector_back_ptr_or_null, vector_create, vector_empty,
    vector_pop, vector_push, vector_element_size, Vector,
};
use std::sync::Mutex;
use lazy_static::lazy_static;

// Re-export Node and NodeBinded so other modules can use them
pub use crate::compiler::Node as NodeType;
pub use crate::compiler::NodeBinded as NodeBindedType;

lazy_static! {
    static ref NODES: Mutex<Vec<Node>> = Mutex::new(Vec::new());
}
lazy_static! {
    static ref NODE_VECTOR: Mutex<Option<Vector>> = Mutex::new(None);
}
lazy_static! {
    static ref NODE_VECTOR_ROOT: Mutex<Option<Vector>> = Mutex::new(None);
}

fn encode_index(idx: u64) -> [u8; 8] {
    idx.to_le_bytes()
}

fn decode_index(bytes: &[u8]) -> Option<u64> {
    if bytes.len() < 8 {
        return None;
    }
    Some(u64::from_le_bytes(bytes[0..8].try_into().ok()?))
}

pub fn node_set_vector(vec: Vector, root_vec: Vector) {
    let mut nv = NODE_VECTOR.lock().unwrap();
    *nv = Some(vec);
    let mut nr = NODE_VECTOR_ROOT.lock().unwrap();
    *nr = Some(root_vec);
}

pub fn node_push(node: &Node) {
    let mut nodes = NODES.lock().unwrap();
    let idx = nodes.len() as u64;
    nodes.push(node.clone());
    let bytes = encode_index(idx);
    let mut nv = NODE_VECTOR.lock().unwrap();
    if let Some(ref mut v) = *nv {
        vector_push(v, &bytes);
    }
}

pub fn node_peek_or_null() -> Option<Node> {
    let mut nv = NODE_VECTOR.lock().unwrap();
    let v = nv.as_mut()?;
    let back = vector_back_ptr_or_null(v)?;
    let idx = decode_index(back)? as usize;
    let nodes = NODES.lock().unwrap();
    nodes.get(idx).cloned()
}

pub fn node_peek() -> Node {
    node_peek_or_null().unwrap_or_default()
}

pub fn node_pop() -> Node {
    let last_idx;
    {
        let mut nv = NODE_VECTOR.lock().unwrap();
        let v = nv.as_mut().expect("node_vector not set");
        let back = vector_back_ptr(v).expect("node_vector empty");
        last_idx = decode_index(back).expect("bad index") as usize;
        vector_pop(v);
    }

    {
        let mut nr = NODE_VECTOR_ROOT.lock().unwrap();
        if let Some(ref mut rv) = *nr {
            if !vector_empty(rv) {
                let root_back = vector_back_ptr(rv).expect("root back");
                let root_idx = decode_index(root_back).unwrap_or(u64::MAX) as usize;
                if root_idx == last_idx {
                    vector_pop(rv);
                }
            }
        }
    }

    let nodes = NODES.lock().unwrap();
    nodes.get(last_idx).cloned().unwrap_or_default()
}

pub fn node_create(template: &Node) -> Node {
    let node = template.clone();
    node_push(&node);
    node
}
