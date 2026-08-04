use crate::compiler::Pos;
use crate::vector::Vector;
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
    let mut a = [0u8; 8];
    a.copy_from_slice(&bytes[..8]);
    Some(u64::from_le_bytes(a))
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
    let mut nodes = NODES.lock().unwrap();
    let idx = nodes.len() as u64;
    nodes.push(node.clone());
    drop(nodes);

    let mut nv = NODE_VECTOR.lock().unwrap();
    if let Some(v) = nv.as_mut() {
        let bytes = encode_index(idx);
        crate::vector::vector_push(v, &bytes);
    }
}

/// Returns the last node or None if empty (like node_peek_or_null).
pub fn node_peek_or_null() -> Option<Node> {
    let mut nv = NODE_VECTOR.lock().unwrap();
    let v = nv.as_mut()?;
    let back = crate::vector::vector_back_or_null(v)?;
    let idx = decode_index(back)?;
    drop(nv);
    let nodes = NODES.lock().unwrap();
    nodes.get(idx as usize).cloned()
}

/// Returns the last Node. If none, returns default. Equivalent to node_peek in the original C code.
pub fn node_peek() -> Node {
    node_peek_or_null().unwrap_or_default()
}

/// Pops the last node. Also checks if the same index matches node_vector_root top, popping that too.
pub fn node_pop() -> Node {
    let mut nv = NODE_VECTOR.lock().unwrap();
    let v = match nv.as_mut() {
        Some(v) => v,
        None => return Node::default(),
    };
    let back_bytes_opt = crate::vector::vector_back_or_null(v).map(|b| b.to_vec());
    let last_idx = back_bytes_opt.as_ref().and_then(|b| decode_index(b));

    // last_node_root: peek root (when not empty)
    let mut nvr = NODE_VECTOR_ROOT.lock().unwrap();
    let root_idx = match nvr.as_mut() {
        Some(rv) if !crate::vector::vector_empty(rv) => {
            crate::vector::vector_back_or_null(rv).and_then(|b| decode_index(b))
        }
        _ => None,
    };

    if !crate::vector::vector_empty(v) {
        crate::vector::vector_pop(v);
    }

    if last_idx.is_some() && last_idx == root_idx {
        if let Some(rv) = nvr.as_mut() {
            if !crate::vector::vector_empty(rv) {
                crate::vector::vector_pop(rv);
            }
        }
    }

    drop(nvr);
    drop(nv);

    let nodes = NODES.lock().unwrap();
    last_idx
        .and_then(|i| nodes.get(i as usize).cloned())
        .unwrap_or_default()
}

/// Creates a new node from a template node, pushing it onto node_vector and returning the clone.
pub fn node_create(template: &Node) -> Node {
    node_push(template);
    template.clone()
}
