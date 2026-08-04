use crate::compiler::{Node, Token};

// Constants
pub const VECTOR_ELEMENT_INCREMENT: usize = 20;
pub const VECTOR_FLAG_PEEK_DECREMENT: i32 = 0b00000001;
// Structs
/// A safe, idiomatic representation of the original `struct vector`.
/// Holds elements as either raw byte rows, tokens, or nodes depending on usage.
#[derive(Debug, Default, Clone)]
pub struct Vector {
    pub(crate) data: Vec<Vec<u8>>,
    pub(crate) tokens: Vec<Token>,
    pub(crate) nodes: Vec<Node>,
    pub(crate) pindex: i32,
    pub(crate) flags: i32,
    pub(crate) esize: usize,
    pub(crate) saves: Vec<(i32, usize, usize, usize)>,
}

impl Vector {
    pub fn count_total(&self) -> usize {
        self.data.len() + self.tokens.len() + self.nodes.len()
    }
}

// --- Generic byte-slice helpers ---
/// Creates a new vector with elements of size `esize`.
pub fn vector_create(esize: usize) -> Vector {
    Vector {
        data: Vec::new(),
        tokens: Vec::new(),
        nodes: Vec::new(),
        pindex: 0,
        flags: 0,
        esize,
        saves: Vec::new(),
    }
}

/// Frees the given vector (in Rust, typically done by dropping).
pub fn vector_free(_vector: Vector) {}

/// Returns a reference to the element at the given index, if in range.
pub fn vector_at(vector: &mut Vector, index: i32) -> Option<&mut [u8]> {
    if index < 0 {
        return None;
    }
    let i = index as usize;
    vector.data.get_mut(i).map(|v| v.as_mut_slice())
}

pub fn vector_peek_ptr_at(vector: &mut Vector, index: i32) -> Option<&mut [u8]> {
    vector_at(vector, index)
}

pub fn vector_peek_no_increment(vector: &mut Vector) -> Option<&mut [u8]> {
    let pi = vector.pindex;
    if pi < 0 || (pi as usize) >= vector.data.len() {
        return None;
    }
    vector.data.get_mut(pi as usize).map(|v| v.as_mut_slice())
}

pub fn vector_peek(vector: &mut Vector) -> Option<&mut [u8]> {
    let pi = vector.pindex;
    if pi < 0 || (pi as usize) >= vector.data.len() {
        return None;
    }
    if vector.flags & VECTOR_FLAG_PEEK_DECREMENT != 0 {
        vector.pindex -= 1;
    } else {
        vector.pindex += 1;
    }
    vector.data.get_mut(pi as usize).map(|v| v.as_mut_slice())
}

pub fn vector_peek_at(vector: &mut Vector, index: i32) -> Option<&mut [u8]> {
    vector_at(vector, index)
}

pub fn vector_set_flag(vector: &mut Vector, flag: i32) {
    vector.flags |= flag;
}

pub fn vector_unset_flag(vector: &mut Vector, flag: i32) {
    vector.flags &= !flag;
}

pub fn vector_pop_last_peek(vector: &mut Vector) {
    if vector.pindex >= 1 {
        vector_pop_at(vector, vector.pindex - 1);
    }
}

pub fn vector_peek_ptr(vector: &mut Vector) -> Option<&mut [u8]> {
    vector_peek(vector)
}

pub fn vector_set_peek_pointer(vector: &mut Vector, index: i32) {
    vector.pindex = index;
}

pub fn vector_set_peek_pointer_end(vector: &mut Vector) {
    let n = if !vector.tokens.is_empty() {
        vector.tokens.len()
    } else if !vector.nodes.is_empty() {
        vector.nodes.len()
    } else {
        vector.data.len()
    };
    vector.pindex = (n as i32) - 1;
}

pub fn vector_push(vector: &mut Vector, elem: &[u8]) {
    let mut entry = vec![0u8; vector.esize.max(elem.len())];
    let n = elem.len().min(entry.len());
    entry[..n].copy_from_slice(&elem[..n]);
    vector.data.push(entry);
}

pub fn vector_push_at(vector: &mut Vector, index: i32, ptr: &[u8]) {
    let mut entry = vec![0u8; vector.esize.max(ptr.len())];
    let n = ptr.len().min(entry.len());
    entry[..n].copy_from_slice(&ptr[..n]);
    let i = (index.max(0) as usize).min(vector.data.len());
    vector.data.insert(i, entry);
}

pub fn vector_pop(vector: &mut Vector) {
    if !vector.tokens.is_empty() {
        vector.tokens.pop();
    } else if !vector.nodes.is_empty() {
        vector.nodes.pop();
    } else {
        vector.data.pop();
    }
}

pub fn vector_peek_pop(vector: &mut Vector) {
    vector_pop_at(vector, vector.pindex);
}

pub fn vector_back(vector: &mut Vector) -> Option<&mut [u8]> {
    vector.data.last_mut().map(|v| v.as_mut_slice())
}

pub fn vector_back_or_null(vector: &mut Vector) -> Option<&mut [u8]> {
    vector_back(vector)
}

pub fn vector_back_ptr(vector: &mut Vector) -> Option<&mut [u8]> {
    vector_back(vector)
}

pub fn vector_back_ptr_or_null(vector: &mut Vector) -> Option<&mut [u8]> {
    vector_back(vector)
}

pub fn vector_string(vec: &Vector) -> Option<&str> {
    vec.data.first().and_then(|v| std::str::from_utf8(v).ok())
}

pub fn vector_empty(vector: &Vector) -> bool {
    vector.data.is_empty() && vector.tokens.is_empty() && vector.nodes.is_empty()
}

pub fn vector_clear(vector: &mut Vector) {
    vector.data.clear();
    vector.tokens.clear();
    vector.nodes.clear();
    vector.pindex = 0;
}

pub fn vector_count(vector: &Vector) -> i32 {
    if !vector.tokens.is_empty() {
        vector.tokens.len() as i32
    } else if !vector.nodes.is_empty() {
        vector.nodes.len() as i32
    } else {
        vector.data.len() as i32
    }
}

pub fn vector_fread(_vector: &mut Vector, _amount: i32, _fp: std::fs::File) -> i32 {
    0
}

pub fn vector_data_ptr(vector: &Vector) -> &[u8] {
    if let Some(first) = vector.data.first() {
        first.as_slice()
    } else {
        &[]
    }
}

pub fn vector_insert(vector_dst: &mut Vector, vector_src: &Vector, dst_index: i32) -> i32 {
    if vector_dst.esize != vector_src.esize {
        return -1;
    }
    let mut i = (dst_index.max(0) as usize).min(vector_dst.data.len());
    for entry in &vector_src.data {
        vector_dst.data.insert(i, entry.clone());
        i += 1;
    }
    0
}

pub fn vector_pop_at_data_address(_vector: &mut Vector, _address: *const u8) -> i32 {
    0
}

pub fn vector_pop_value(vector: &mut Vector, val: &[u8]) -> i32 {
    if let Some(pos) = vector.data.iter().position(|v| v.as_slice() == val) {
        vector.data.remove(pos);
        return pos as i32;
    }
    -1
}

pub fn vector_pop_at(vector: &mut Vector, index: i32) {
    if index < 0 {
        return;
    }
    let i = index as usize;
    if i < vector.data.len() {
        vector.data.remove(i);
    }
}

pub fn vector_peek_back(vector: &mut Vector) {
    vector.pindex -= 1;
}

pub fn vector_current_index(vector: &Vector) -> i32 {
    if !vector.tokens.is_empty() {
        vector.tokens.len() as i32
    } else if !vector.nodes.is_empty() {
        vector.nodes.len() as i32
    } else {
        vector.data.len() as i32
    }
}

pub fn vector_save(vector: &mut Vector) {
    vector.saves.push((
        vector.pindex,
        vector.data.len(),
        vector.tokens.len(),
        vector.nodes.len(),
    ));
}

pub fn vector_restore(vector: &mut Vector) {
    if let Some((pidx, _, _, _)) = vector.saves.pop() {
        vector.pindex = pidx;
    }
}

pub fn vector_save_purge(vector: &mut Vector) {
    vector.saves.pop();
}

pub fn vector_element_size(vector: &Vector) -> usize {
    vector.esize
}

pub fn vector_clone(vector: &Vector) -> Vector {
    vector.clone()
}

// --- Typed helpers (Token / Node) ---
pub fn vector_push_token(vector: &mut Vector, t: Token) {
    vector.tokens.push(t);
}

pub fn vector_pop_token(vector: &mut Vector) -> Option<Token> {
    vector.tokens.pop()
}

pub fn vector_back_token(vector: &Vector) -> Option<&Token> {
    vector.tokens.last()
}

pub fn vector_back_token_mut(vector: &mut Vector) -> Option<&mut Token> {
    vector.tokens.last_mut()
}

pub fn vector_peek_token_no_increment(vector: &Vector) -> Option<&Token> {
    let pi = vector.pindex;
    if pi < 0 {
        return None;
    }
    vector.tokens.get(pi as usize)
}

pub fn vector_peek_token(vector: &mut Vector) -> Option<Token> {
    let pi = vector.pindex;
    if pi < 0 || (pi as usize) >= vector.tokens.len() {
        return None;
    }
    let t = vector.tokens[pi as usize].clone();
    if vector.flags & VECTOR_FLAG_PEEK_DECREMENT != 0 {
        vector.pindex -= 1;
    } else {
        vector.pindex += 1;
    }
    Some(t)
}

pub fn vector_token_count(vector: &Vector) -> usize {
    vector.tokens.len()
}

pub fn vector_push_node(vector: &mut Vector, n: Node) {
    vector.nodes.push(n);
}

pub fn vector_back_node(vector: &Vector) -> Option<&Node> {
    vector.nodes.last()
}
