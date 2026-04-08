// Constants
pub const FNV_PRIME: u64 = 1099511628211;
pub const FNV_BASE: u64 = 14695981039346656037;
pub const HAMT_NODE_T_FLAG: u32 = 1;
pub const KEY_VALUE_T_FLAG: u32 = 0;
pub const CHUNK_SIZE: usize = 6;
// Type Definitions
pub type HashFn<T> = fn(&mut T) -> u32;
pub type EqualsFn<T> = fn(&mut T, &mut T) -> bool;
pub type StrFn<T> = fn(&mut T) -> String;
pub type DeallocateFn<T> = fn(&mut T);
pub struct KeyValue<'a, T, U> {
    pub key: &'a mut T,
    pub value: &'a mut U,
}
impl<T, U> KeyValue<'_, T, U> {}
pub enum HamtNode<'a, T, U> {
    Leaf(Option<KeyValue<'a, T, U>>),
    Sub(SubNode<'a, T, U>),
}
impl<'a, T, U> HamtNode<'a, T, U> {
    pub fn get_children_pointer(&mut self) -> Self {
        match self {
            HamtNode::Sub(sub) => {
                let children = std::mem::take(&mut sub.children);
                HamtNode::Sub(SubNode {
                    bitmap: sub.bitmap,
                    children,
                })
            }
            _ => HamtNode::Leaf(None),
        }
    }
    pub fn is_leaf(&mut self) -> bool {
        matches!(self, HamtNode::Leaf(_))
    }
    pub fn hamt_node_search(
        &mut self,
        hash: u32,
        lvl: i32,
        key: &mut T,
        equals_fn: EqualsFn<T>,
    ) -> Self {
        match self {
            HamtNode::Leaf(Some(kv)) => {
                if equals_fn(kv.key, key) {
                    HamtNode::Leaf(None) // found
                } else {
                    HamtNode::Sub(SubNode {
                        bitmap: 0,
                        children: Vec::new(),
                    }) // not found
                }
            }
            HamtNode::Leaf(None) => HamtNode::Sub(SubNode {
                bitmap: 0,
                children: Vec::new(),
            }),
            HamtNode::Sub(sub) => {
                let symbol = hamt_get_symbol_impl(hash, lvl);
                let shifted = sub.bitmap.wrapping_shr(symbol);
                if (shifted & 1) != 0 {
                    let pos = (shifted >> 1).count_ones() as usize;
                    if pos < sub.children.len() {
                        return sub.children[pos].hamt_node_search(hash, lvl + 1, key, equals_fn);
                    }
                }
                HamtNode::Sub(SubNode {
                    bitmap: 0,
                    children: Vec::new(),
                })
            }
        }
    }
    pub fn hamt_node_insert(
        &mut self,
        hash: u32,
        lvl: i32,
        mut key: &'a mut T,
        mut value: &'a mut U,
        hash_fn: HashFn<T>,
        equals_fn: EqualsFn<T>,
        conflict_kv: &mut KeyValue<'a, T, U>,
    ) -> bool {
        if (lvl as usize) * CHUNK_SIZE > 32 {
            return false;
        }

        match self {
            HamtNode::Leaf(ref mut leaf_opt) => {
                if let Some(kv) = leaf_opt {
                    if equals_fn(kv.key, &mut *key) {
                        // Existing key: swap old into conflict_kv, put new in node
                        std::mem::swap(&mut kv.key, &mut conflict_kv.key);
                        std::mem::swap(&mut kv.value, &mut conflict_kv.value);
                        std::mem::swap(&mut kv.key, &mut key);
                        std::mem::swap(&mut kv.value, &mut value);
                        return false;
                    }
                    // Different key: convert leaf to sub-node
                    let original_hash = hash_fn(kv.key);
                    let original_symbol = hamt_get_symbol_impl(original_hash, lvl);

                    let existing_kv = leaf_opt.take().unwrap();
                    let child = HamtNode::Leaf(Some(existing_kv));

                    *self = HamtNode::Sub(SubNode {
                        bitmap: 1u32.wrapping_shl(original_symbol),
                        children: vec![child],
                    });

                    return self.hamt_node_insert(hash, lvl, key, value, hash_fn, equals_fn, conflict_kv);
                }
                false
            }
            HamtNode::Sub(ref mut sub) => {
                let symbol = hamt_get_symbol_impl(hash, lvl);
                let shifted = sub.bitmap.wrapping_shr(symbol);
                let child_exists = (shifted & 1) != 0;

                if child_exists {
                    let pos = (shifted >> 1).count_ones() as usize;
                    return sub.children[pos].hamt_node_insert(
                        hash, lvl + 1, key, value, hash_fn, equals_fn, conflict_kv,
                    );
                }
                // Free slot
                let before = (shifted >> 1).count_ones() as usize;
                sub.bitmap |= 1u32.wrapping_shl(symbol);
                sub.children.insert(before, HamtNode::Leaf(Some(KeyValue { key, value })));
                true
            }
        }
    }
    pub fn hamt_node_remove(
        &mut self,
        hash: u32,
        lvl: i32,
        key: &mut T,
        equals_fn: EqualsFn<T>,
        removed_kv: &mut KeyValue<'a, T, U>,
    ) -> bool {
        let mut removed = false;

        if let HamtNode::Sub(ref mut sub) = self {
            let symbol = hamt_get_symbol_impl(hash, lvl);
            let shifted = sub.bitmap.wrapping_shr(symbol);
            if (shifted & 1) != 0 {
                let pos = (shifted >> 1).count_ones() as usize;

                match &mut sub.children[pos] {
                    HamtNode::Leaf(Some(leaf_kv)) => {
                        if equals_fn(leaf_kv.key, key) {
                            std::mem::swap(&mut leaf_kv.key, &mut removed_kv.key);
                            std::mem::swap(&mut leaf_kv.value, &mut removed_kv.value);
                            sub.bitmap &= !1u32.wrapping_shl(symbol);
                            sub.children.remove(pos);
                            removed = true;
                        }
                    }
                    child @ HamtNode::Sub(_) => {
                        removed = child.hamt_node_remove(hash, lvl + 1, key, equals_fn, removed_kv);
                    }
                    _ => {}
                }
            }

            // Collapse if only one leaf child remains
            if sub.children.len() == 1 && matches!(&sub.children[0], HamtNode::Leaf(_)) {
                let only_child = sub.children.remove(0);
                *self = only_child;
            }
        }

        removed
    }
    pub fn hamt_node_destroy(
        &mut self,
        deallocate_fn_key: DeallocateFn<T>,
        deallocate_fn_val: DeallocateFn<U>,
    ) {
        match self {
            HamtNode::Leaf(Some(kv)) => {
                deallocate_fn_key(kv.key);
                deallocate_fn_val(kv.value);
            }
            HamtNode::Sub(sub) => {
                for child in sub.children.iter_mut() {
                    child.hamt_node_destroy(deallocate_fn_key, deallocate_fn_val);
                }
            }
            _ => {}
        }
    }
    pub fn hamt_node_print(&mut self, lvl: i32, str_fn_key: StrFn<T>, str_fn_val: StrFn<U>) {
        let indent = "  ".repeat(lvl as usize);
        match self {
            HamtNode::Leaf(Some(kv)) => {
                println!("{}{{{} -> {}}}", indent, str_fn_key(kv.key), str_fn_val(kv.value));
            }
            HamtNode::Sub(sub) => {
                println!("{}bitmap: {:08x}", indent, sub.bitmap);
                for child in sub.children.iter_mut() {
                    child.hamt_node_print(lvl + 1, str_fn_key, str_fn_val);
                }
            }
            _ => {}
        }
    }
}
pub struct SubNode<'a, T, U> {
    pub bitmap: u32,
    pub children: Vec<HamtNode<'a, T, U>>,
}
pub struct Hamt<'a, T, U> {
    pub root: Option<Box<HamtNode<'a, T, U>>>,
    pub size: i32,
    pub hash_fn: HashFn<T>,
    pub equals_fn: EqualsFn<T>,
}
impl<'a, T, U> Hamt<'a, T, U> {
    pub fn new_hamt(hash_fn: HashFn<T>, equals_fn: EqualsFn<T>) -> Self {
        Hamt {
            root: Some(Box::new(HamtNode::Leaf(None))),
            size: 0,
            hash_fn,
            equals_fn,
        }
    }
    pub fn hamt_size(&self) -> i32 {
        self.size
    }
    pub fn hamt_set(
        &mut self,
        key: &'a mut T,
        value: &'a mut U,
        conflict_kv: &mut KeyValue<'a, T, U>,
    ) -> bool {
        let hash = (self.hash_fn)(key);
        if self.size == 0 {
            self.root = Some(Box::new(HamtNode::Leaf(Some(KeyValue { key, value }))));
            self.size = 1;
            return false;
        }
        let hash_fn = self.hash_fn;
        let equals_fn = self.equals_fn;
        let root = self.root.as_mut().unwrap();
        let inserted = root.hamt_node_insert(hash, 0, key, value, hash_fn, equals_fn, conflict_kv);
        if inserted {
            self.size += 1;
        }
        !inserted
    }
    pub fn hamt_search(&mut self, key: &mut T) -> bool {
        if self.size == 0 {
            return false;
        }
        let hash = (self.hash_fn)(key);
        let equals_fn = self.equals_fn;
        let root = self.root.as_mut().unwrap();
        let result = root.hamt_node_search(hash, 0, key, equals_fn);
        matches!(result, HamtNode::Leaf(None))
    }
    pub fn hamt_remove(&mut self, key: &mut T, removed_kv: &mut KeyValue<'a, T, U>) -> bool {
        if self.size == 0 {
            return false;
        }
        let hash = (self.hash_fn)(key);
        let equals_fn = self.equals_fn;
        let mut removed = false;

        if self.size == 1 {
            let root = self.root.as_mut().unwrap();
            if let HamtNode::Leaf(Some(kv)) = root.as_mut() {
                if equals_fn(kv.key, key) {
                    std::mem::swap(&mut kv.key, &mut removed_kv.key);
                    std::mem::swap(&mut kv.value, &mut removed_kv.value);
                    removed = true;
                }
            }
        } else {
            let root = self.root.as_mut().unwrap();
            removed = root.hamt_node_remove(hash, 0, key, equals_fn, removed_kv);
        }

        if removed {
            self.size -= 1;
        }
        removed
    }
    pub fn hamt_destroy(
        &mut self,
        deallocate_fn: DeallocateFn<T>,
        deallocate_fn_val: DeallocateFn<U>,
    ) {
        if self.size > 0 {
            if let Some(ref mut root) = self.root {
                root.hamt_node_destroy(deallocate_fn, deallocate_fn_val);
            }
        }
        self.root = None;
    }
    pub fn hamt_print(&mut self, str_fn_key: StrFn<T>, str_fn_val: StrFn<U>) {
        if self.size > 0 {
            if let Some(ref mut root) = self.root {
                root.hamt_node_print(0, str_fn_key, str_fn_val);
            }
        } else {
            println!("{{}}");
        }
        println!("---\n");
    }
}
// Function Declarations
pub fn hamt_int_hash<T>(key: &mut T) -> u32 {
    let bytes = unsafe {
        std::slice::from_raw_parts(key as *const T as *const u8, std::mem::size_of::<T>())
    };
    hamt_fnv1_hash_bytes(bytes)
}
pub fn hamt_str_hash<T>(key: &mut T) -> u32 {
    let bytes = unsafe {
        std::slice::from_raw_parts(key as *const T as *const u8, std::mem::size_of::<T>())
    };
    let mut hash = FNV_BASE as u32;
    for &b in bytes {
        if b == 0 {
            break;
        }
        hash = hash.wrapping_mul(FNV_PRIME as u32);
        hash ^= b as i8 as i32 as u32;
    }
    hash
}
pub fn hamt_int_equals<T>(a: &mut T, b: &mut T) -> bool {
    let a_bytes = unsafe {
        std::slice::from_raw_parts(a as *const T as *const u8, std::mem::size_of::<T>())
    };
    let b_bytes = unsafe {
        std::slice::from_raw_parts(b as *const T as *const u8, std::mem::size_of::<T>())
    };
    a_bytes == b_bytes
}
pub fn hamt_str_equals<T>(a: &mut T, b: &mut T) -> bool {
    let a_bytes = unsafe {
        std::slice::from_raw_parts(a as *const T as *const u8, std::mem::size_of::<T>())
    };
    let b_bytes = unsafe {
        std::slice::from_raw_parts(b as *const T as *const u8, std::mem::size_of::<T>())
    };
    a_bytes == b_bytes
}
pub fn hamt_fnv1_hash<T>(_key: &mut T, _len: usize) {
    // No return type in interface; actual hashing done by hamt_int_hash/hamt_str_hash
}
pub fn hamt_get_symbol(_hash: u32, _lvl: i32) {
    // No return type in interface; actual logic in hamt_get_symbol_impl
}

// Internal helpers
fn hamt_fnv1_hash_bytes(bytes: &[u8]) -> u32 {
    let mut hash = FNV_BASE as u32;
    for &b in bytes {
        hash = hash.wrapping_mul(FNV_PRIME as u32);
        hash ^= b as i8 as i32 as u32;
    }
    hash
}

fn hamt_get_symbol_impl(hash: u32, lvl: i32) -> u32 {
    let left = (lvl as u32) * (CHUNK_SIZE as u32);
    let left_plus_chunk = left + CHUNK_SIZE as u32;
    let right = if left_plus_chunk > 32 { 0 } else { 32 - left_plus_chunk };
    hash.wrapping_shl(left).wrapping_shr(right + left)
}
