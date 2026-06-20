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
    let mut arr = [0u8; 8];
    arr.copy_from_slice(&bytes[..8]);
    Some(u64::from_le_bytes(arr))
}

fn lookup_node(bytes: &[u8]) -> Option<Node> {
    let index = decode_index(bytes)? as usize;
    let nodes = NODES.lock().ok()?;
    nodes.get(index).cloned()
}

pub fn node_set_vector(vec: Vector, root_vec: Vector) {
    if let Ok(mut node_vec) = NODE_VECTOR.lock() {
        *node_vec = Some(vec);
    }
    if let Ok(mut root) = NODE_VECTOR_ROOT.lock() {
        *root = Some(root_vec);
    }
}

pub fn node_push(node: &Node) {
    let index = {
        let mut nodes = match NODES.lock() {
            Ok(nodes) => nodes,
            Err(_) => return,
        };
        nodes.push(node.clone());
        nodes.len() as u64 - 1
    };

    if let Ok(mut node_vec) = NODE_VECTOR.lock() {
        if let Some(vec) = node_vec.as_mut() {
            vector_push(vec, &encode_index(index));
        }
    }
}

pub fn node_peek_or_null() -> Option<Node> {
    let mut node_vec = NODE_VECTOR.lock().ok()?;
    let bytes = vector_back_ptr_or_null(node_vec.as_mut()?)?.to_vec();
    lookup_node(&bytes)
}

pub fn node_peek() -> Node {
    node_peek_or_null().unwrap_or_default()
}

pub fn node_pop() -> Node {
    let last_node = {
        let mut node_vec = match NODE_VECTOR.lock() {
            Ok(node_vec) => node_vec,
            Err(_) => return Node::default(),
        };
        let Some(vec) = node_vec.as_mut() else {
            return Node::default();
        };
        let bytes = match vector_back_ptr(vec) {
            Some(bytes) => bytes.to_vec(),
            None => return Node::default(),
        };
        let node = lookup_node(&bytes).unwrap_or_default();
        vector_pop(vec);
        (node, bytes)
    };

    if let Ok(mut root_vec) = NODE_VECTOR_ROOT.lock() {
        if let Some(root) = root_vec.as_mut() {
            if let Some(root_bytes) = vector_back_ptr_or_null(root) {
                if root_bytes == last_node.1.as_slice() {
                    vector_pop(root);
                }
            }
        }
    }

    last_node.0
}

pub fn node_create(template: &Node) -> Node {
    let node = template.clone();
    node_push(&node);
    node
}
