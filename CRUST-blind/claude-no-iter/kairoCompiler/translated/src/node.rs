use crate::compiler::Pos;
use crate::vector::{
    vector_back, vector_back_or_null, vector_empty, vector_pop, vector_push, Vector,
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
    let idx = {
        let mut nodes = NODES.lock().unwrap();
        nodes.push(node.clone());
        (nodes.len() - 1) as u64
    };
    let bytes = encode_index(idx);
    let mut guard = NODE_VECTOR.lock().unwrap();
    if let Some(vec) = guard.as_mut() {
        vector_push(vec, &bytes);
    }
}

/// Returns the last node or None if empty (like node_peek_or_null).
pub fn node_peek_or_null() -> Option<Node> {
    let mut guard = NODE_VECTOR.lock().unwrap();
    let vec = guard.as_mut()?;
    let slot = vector_back_or_null(vec)?;
    let idx = decode_index(slot)?;
    let nodes = NODES.lock().unwrap();
    nodes.get(idx as usize).cloned()
}

/// Returns the last Node. If none, returns default. Equivalent to node_peek in the original C code.
pub fn node_peek() -> Node {
    let mut guard = NODE_VECTOR.lock().unwrap();
    let vec = match guard.as_mut() {
        Some(v) => v,
        None => return Node::default(),
    };
    let slot = match vector_back(vec) {
        Some(s) => s,
        None => return Node::default(),
    };
    let idx = match decode_index(slot) {
        Some(i) => i,
        None => return Node::default(),
    };
    let nodes = NODES.lock().unwrap();
    nodes.get(idx as usize).cloned().unwrap_or_default()
}

/// Pops the last node. Also checks if the same index matches node_vector_root top, popping that too.
pub fn node_pop() -> Node {
    let last_idx = {
        let mut guard = NODE_VECTOR.lock().unwrap();
        let vec = match guard.as_mut() {
            Some(v) => v,
            None => return Node::default(),
        };
        match vector_back_or_null(vec) {
            Some(slot) => decode_index(slot),
            None => None,
        }
    };

    let root_idx: Option<u64> = {
        let mut guard = NODE_VECTOR_ROOT.lock().unwrap();
        match guard.as_mut() {
            Some(v) if !vector_empty(v) => match vector_back_or_null(v) {
                Some(slot) => decode_index(slot),
                None => None,
            },
            _ => None,
        }
    };

    {
        let mut guard = NODE_VECTOR.lock().unwrap();
        if let Some(vec) = guard.as_mut() {
            if !vector_empty(vec) {
                vector_pop(vec);
            }
        }
    }

    if last_idx.is_some() && last_idx == root_idx {
        let mut guard = NODE_VECTOR_ROOT.lock().unwrap();
        if let Some(vec) = guard.as_mut() {
            if !vector_empty(vec) {
                vector_pop(vec);
            }
        }
    }

    let nodes = NODES.lock().unwrap();
    match last_idx {
        Some(i) => nodes.get(i as usize).cloned().unwrap_or_default(),
        None => Node::default(),
    }
}

/// Creates a new node from a template node, pushing it onto node_vector and returning the clone.
pub fn node_create(template: &Node) -> Node {
    let new_node = template.clone();
    node_push(&new_node);
    new_node
}

/// Resets all global node state. Useful when running tests sequentially.
pub fn node_reset_globals_for_tests() {
    *NODES.lock().unwrap() = Vec::new();
    *NODE_VECTOR.lock().unwrap() = None;
    *NODE_VECTOR_ROOT.lock().unwrap() = None;
}

/// Borrow the underlying global node store as a clone (used in parser.rs for synchronisation).
pub fn node_take_vector() -> Option<Vector> {
    NODE_VECTOR.lock().unwrap().take()
}

/// Restore a previously-taken global node vector.
pub fn node_replace_vector(v: Option<Vector>) {
    *NODE_VECTOR.lock().unwrap() = v;
}

/// Borrow the global root node vector.
pub fn node_take_root_vector() -> Option<Vector> {
    NODE_VECTOR_ROOT.lock().unwrap().take()
}

/// Restore the previously-taken global root node vector.
pub fn node_replace_root_vector(v: Option<Vector>) {
    *NODE_VECTOR_ROOT.lock().unwrap() = v;
}
