use crate::compiler::Pos;
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

// Global stack of nodes used by node_push / node_peek / node_pop.
lazy_static! {
    static ref NODE_STACK: Mutex<Vec<Node>> = Mutex::new(Vec::new());
    static ref NODE_ROOT_STACK: Mutex<Vec<Node>> = Mutex::new(Vec::new());
}

/// Sets the two global vectors used for node push/pop. In our safe Rust
/// translation we only need to (re)initialize the internal stacks.
pub fn node_set_vector(_vec: crate::vector::Vector, _root_vec: crate::vector::Vector) {
    NODE_STACK.lock().unwrap().clear();
    NODE_ROOT_STACK.lock().unwrap().clear();
}

/// Pushes a node onto the global node stack.
pub fn node_push(node: &Node) {
    NODE_STACK.lock().unwrap().push(node.clone());
}

/// Returns the last node or None if empty (like node_peek_or_null).
pub fn node_peek_or_null() -> Option<Node> {
    NODE_STACK.lock().unwrap().last().cloned()
}

/// Returns the last Node. If none, returns default. Equivalent to node_peek in the original C code.
pub fn node_peek() -> Node {
    NODE_STACK
        .lock()
        .unwrap()
        .last()
        .cloned()
        .unwrap_or_default()
}

/// Pops the last node. Also checks if the same value matches node_root_stack top, popping that too.
pub fn node_pop() -> Node {
    let mut stack = NODE_STACK.lock().unwrap();
    let last = stack.pop().unwrap_or_default();
    let mut root = NODE_ROOT_STACK.lock().unwrap();
    if let Some(top_root) = root.last() {
        if same_node(&last, top_root) {
            root.pop();
        }
    }
    last
}

fn same_node(a: &Node, b: &Node) -> bool {
    a.r#type == b.r#type
        && a.flags == b.flags
        && a.cval == b.cval
        && a.sval == b.sval
        && a.inum == b.inum
        && a.lnum == b.lnum
        && a.llnum == b.llnum
}

/// Creates a new node from a template node, pushing it onto the node stack and returning the clone.
pub fn node_create(template: &Node) -> Node {
    let n = template.clone();
    NODE_STACK.lock().unwrap().push(n.clone());
    n
}
