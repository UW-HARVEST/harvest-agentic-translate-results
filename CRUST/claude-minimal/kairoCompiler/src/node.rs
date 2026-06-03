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
    let mut buf = [0u8; 8];
    buf.copy_from_slice(&bytes[..8]);
    Some(u64::from_le_bytes(buf))
}

/// Sets the two global vectors used for node push/pop.
pub fn node_set_vector(vec: Vector, root_vec: Vector) {
    *NODE_VECTOR.lock().unwrap() = Some(vec);
    *NODE_VECTOR_ROOT.lock().unwrap() = Some(root_vec);
}

/// Take the vectors out of the global state (for callers that need them back).
pub fn node_take_vectors() -> (Option<Vector>, Option<Vector>) {
    let v = NODE_VECTOR.lock().unwrap().take();
    let r = NODE_VECTOR_ROOT.lock().unwrap().take();
    (v, r)
}

/// Push the index of the top of node_vector onto node_vector_root.
pub fn node_root_push_top() {
    let mut nv = NODE_VECTOR.lock().unwrap();
    let bytes = match nv.as_mut() {
        Some(vec) => match crate::vector::vector_back_or_null(vec) {
            Some(slot) => slot.to_vec(),
            None => return,
        },
        None => return,
    };
    drop(nv);
    let mut nvr = NODE_VECTOR_ROOT.lock().unwrap();
    if let Some(rvec) = nvr.as_mut() {
        crate::vector::vector_push(rvec, &bytes);
    }
}

/// Pushes a node onto node_vector.
pub fn node_push(node: &Node) {
    let mut nodes = NODES.lock().unwrap();
    let idx = nodes.len() as u64;
    nodes.push(node.clone());
    drop(nodes);

    let bytes = encode_index(idx);
    let mut nv = NODE_VECTOR.lock().unwrap();
    if let Some(vec) = nv.as_mut() {
        crate::vector::vector_push(vec, &bytes);
    }
}

/// Returns the last node or None if empty (like node_peek_or_null).
pub fn node_peek_or_null() -> Option<Node> {
    let mut nv = NODE_VECTOR.lock().unwrap();
    let vec = nv.as_mut()?;
    let back = crate::vector::vector_back_or_null(vec)?;
    let bytes = back.to_vec();
    drop(nv);
    let idx = decode_index(&bytes)?;
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
    let vec = match nv.as_mut() {
        Some(v) => v,
        None => return Node::default(),
    };
    let last_idx = match crate::vector::vector_back_or_null(vec) {
        Some(b) => match decode_index(&b.to_vec()) {
            Some(i) => i,
            None => {
                return Node::default();
            }
        },
        None => return Node::default(),
    };

    // Check root
    let mut nvr = NODE_VECTOR_ROOT.lock().unwrap();
    let root_idx = if let Some(root_vec) = nvr.as_mut() {
        if !crate::vector::vector_empty(root_vec) {
            crate::vector::vector_back_or_null(root_vec)
                .and_then(|b| decode_index(&b.to_vec()))
        } else {
            None
        }
    } else {
        None
    };

    crate::vector::vector_pop(vec);
    drop(nv);

    if Some(last_idx) == root_idx {
        if let Some(root_vec) = nvr.as_mut() {
            crate::vector::vector_pop(root_vec);
        }
    }
    drop(nvr);

    let nodes = NODES.lock().unwrap();
    nodes.get(last_idx as usize).cloned().unwrap_or_default()
}

/// Creates a new node from a template node, pushing it onto node_vector and returning the clone.
pub fn node_create(template: &Node) -> Node {
    let mut node = template.clone();
    // The C code: #warning "we should set the binded owner and binded function here"
    // So this is a placeholder; we leave binded with default None values.
    node_push(&node);
    node
}
