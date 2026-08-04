use crate::compiler::Pos;
use crate::vector::{
    vector_back_ptr, vector_back_ptr_or_null, vector_pop, vector_push, Vector,
};
use lazy_static::lazy_static;
use std::sync::Mutex;

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

lazy_static! {
    static ref NODES: Mutex<Vec<Node>> = Mutex::new(Vec::new());
    static ref NODE_VECTOR: Mutex<Option<Vector>> = Mutex::new(None);
    static ref NODE_VECTOR_ROOT: Mutex<Option<Vector>> = Mutex::new(None);
}

fn encode_index(idx: u64) -> [u8; 8] {
    idx.to_le_bytes()
}

fn decode_index(bytes: &[u8]) -> Option<u64> {
    if bytes.len() < 8 {
        return None;
    }
    let mut raw = [0u8; 8];
    raw.copy_from_slice(&bytes[..8]);
    Some(u64::from_le_bytes(raw))
}

pub fn node_set_vector(vec: Vector, root_vec: Vector) {
    *NODE_VECTOR.lock().expect("node vector mutex poisoned") = Some(vec);
    *NODE_VECTOR_ROOT.lock().expect("node root vector mutex poisoned") = Some(root_vec);
}

pub fn node_push(node: &Node) {
    let mut nodes = NODES.lock().expect("nodes mutex poisoned");
    nodes.push(node.clone());
    let idx = (nodes.len() - 1) as u64;
    drop(nodes);

    if let Some(vec) = NODE_VECTOR.lock().expect("node vector mutex poisoned").as_mut() {
        vector_push(vec, &encode_index(idx));
    }
}

pub fn node_peek_or_null() -> Option<Node> {
    let mut guard = NODE_VECTOR.lock().expect("node vector mutex poisoned");
    let bytes = vector_back_ptr_or_null(guard.as_mut()?)?;
    let idx = decode_index(bytes)? as usize;
    NODES.lock().expect("nodes mutex poisoned").get(idx).cloned()
}

pub fn node_peek() -> Node {
    node_peek_or_null().unwrap_or_default()
}

pub fn node_pop() -> Node {
    let mut vec_guard = NODE_VECTOR.lock().expect("node vector mutex poisoned");
    let mut root_guard = NODE_VECTOR_ROOT.lock().expect("node root vector mutex poisoned");

    let idx = vec_guard
        .as_mut()
        .and_then(vector_back_ptr)
        .and_then(|bytes| decode_index(bytes))
        .unwrap_or(0) as usize;

    let root_idx = root_guard
        .as_mut()
        .and_then(vector_back_ptr_or_null)
        .and_then(|bytes| decode_index(bytes))
        .map(|v| v as usize);

    if let Some(vec) = vec_guard.as_mut() {
        vector_pop(vec);
    }
    if root_idx == Some(idx) {
        if let Some(root) = root_guard.as_mut() {
            vector_pop(root);
        }
    }

    NODES
        .lock()
        .expect("nodes mutex poisoned")
        .get(idx)
        .cloned()
        .unwrap_or_default()
}

pub fn node_create(template: &Node) -> Node {
    let node = template.clone();
    node_push(&node);
    node
}

pub fn current_vectors() -> (Option<Vector>, Option<Vector>) {
    (
        NODE_VECTOR.lock().expect("node vector mutex poisoned").clone(),
        NODE_VECTOR_ROOT.lock().expect("node root vector mutex poisoned").clone(),
    )
}

pub fn last_node_index() -> Option<u64> {
    let guard = NODES.lock().expect("nodes mutex poisoned");
    if guard.is_empty() {
        None
    } else {
        Some((guard.len() - 1) as u64)
    }
}
