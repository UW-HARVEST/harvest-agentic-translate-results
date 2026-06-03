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

// Conversions between this Node and the equivalent type in `crate::compiler`.
impl From<&crate::compiler::Node> for Node {
    fn from(n: &crate::compiler::Node) -> Self {
        Node {
            r#type: n.r#type,
            flags: n.flags,
            pos: n.pos.clone(),
            binded: NodeBinded {
                owner: n.binded.owner.as_ref().map(|b| Box::new(Node::from(b.as_ref()))),
                function: n
                    .binded
                    .function
                    .as_ref()
                    .map(|b| Box::new(Node::from(b.as_ref()))),
            },
            cval: n.cval,
            sval: n.sval.clone(),
            inum: n.inum,
            lnum: n.lnum,
            llnum: n.llnum,
        }
    }
}

impl From<&Node> for crate::compiler::Node {
    fn from(n: &Node) -> Self {
        crate::compiler::Node {
            r#type: n.r#type,
            flags: n.flags,
            pos: n.pos.clone(),
            binded: crate::compiler::NodeBinded {
                owner: n
                    .binded
                    .owner
                    .as_ref()
                    .map(|b| Box::new(crate::compiler::Node::from(b.as_ref()))),
                function: n
                    .binded
                    .function
                    .as_ref()
                    .map(|b| Box::new(crate::compiler::Node::from(b.as_ref()))),
            },
            cval: n.cval,
            sval: n.sval.clone(),
            inum: n.inum,
            lnum: n.lnum,
            llnum: n.llnum,
        }
    }
}

/// Global list of all nodes. We store them here so we can push references (indices) into the vectors.
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

/// Pushes the given Node into the global storage and returns its index.
fn store_node(node: Node) -> u64 {
    let mut nodes = NODES.lock().unwrap();
    nodes.push(node);
    (nodes.len() - 1) as u64
}

fn read_node(idx: u64) -> Option<Node> {
    let nodes = NODES.lock().unwrap();
    nodes.get(idx as usize).cloned()
}

/// Pushes an index onto the given vector.
fn vec_push_index(vec: &mut Vector, idx: u64) {
    crate::vector::vector_push(vec, &encode_index(idx));
}

/// Reads the back-most index from the vector, if any.
fn vec_back_index(vec: &Vector) -> Option<u64> {
    if vec.count == 0 {
        return None;
    }
    let off = ((vec.rindex - 1) as usize) * vec.esize;
    if off + 8 > vec.data.len() {
        return None;
    }
    decode_index(&vec.data[off..off + 8])
}

/// Sets the two global vectors used for node push/pop.
pub fn node_set_vector(vec: Vector, root_vec: Vector) {
    *NODE_VECTOR.lock().unwrap() = Some(vec);
    *NODE_VECTOR_ROOT.lock().unwrap() = Some(root_vec);
}

/// Pushes a node onto node_vector.
pub fn node_push(node: &Node) {
    let idx = store_node(node.clone());
    let mut guard = NODE_VECTOR.lock().unwrap();
    if let Some(vec) = guard.as_mut() {
        vec_push_index(vec, idx);
    }
}

/// Returns the last node or None if empty (like node_peek_or_null).
pub fn node_peek_or_null() -> Option<Node> {
    let guard = NODE_VECTOR.lock().unwrap();
    let vec = guard.as_ref()?;
    let idx = vec_back_index(vec)?;
    read_node(idx)
}

/// Returns the last Node, or `None` if empty. Public, used by parser.rs.
pub fn node_peek_opt() -> Option<Node> {
    node_peek_or_null()
}

/// Returns the last Node. If none, returns default. Equivalent to node_peek in the original C code.
pub fn node_peek() -> Node {
    node_peek_or_null().unwrap_or_default()
}

/// Returns the last Node and removes it from the vector. Returns default if empty.
pub fn node_pop() -> Node {
    node_pop_opt().unwrap_or_default()
}

/// Pops the last node. Also checks if the same index matches node_vector_root top, popping that too.
pub fn node_pop_opt() -> Option<Node> {
    let last_idx;
    {
        let mut guard = NODE_VECTOR.lock().unwrap();
        let vec = guard.as_mut()?;
        last_idx = vec_back_index(vec)?;
        crate::vector::vector_pop(vec);
    }

    {
        let mut root_guard = NODE_VECTOR_ROOT.lock().unwrap();
        if let Some(root_vec) = root_guard.as_mut() {
            if let Some(root_idx) = vec_back_index(root_vec) {
                if root_idx == last_idx {
                    crate::vector::vector_pop(root_vec);
                }
            }
        }
    }
    read_node(last_idx)
}

/// Creates a new node from a template node, pushing it onto node_vector and returning the clone.
pub fn node_create(template: &Node) -> Node {
    let cloned = template.clone();
    node_push(&cloned);
    cloned
}
