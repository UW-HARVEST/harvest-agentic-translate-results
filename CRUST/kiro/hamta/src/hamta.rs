// Constants
pub const FNV_PRIME: u64 = 1099511628211;
pub const FNV_BASE: u64 = 14695981039346656037;
pub const HAMT_NODE_T_FLAG: u32 = 1;
pub const KEY_VALUE_T_FLAG: u32 = 0;
pub const CHUNK_SIZE: usize = 5;

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

pub enum HamtNode<T, U> {
    Leaf { key: T, value: U },
    Sub { bitmap: u32, children: Vec<HamtNode<T, U>> },
}

pub struct SubNode<'a, T, U> {
    pub bitmap: u32,
    pub children: Option<Box<HamtNode<T, U>>>,
    _marker: std::marker::PhantomData<&'a ()>,
}

pub struct Hamt<'a, T, U> {
    pub root: Option<Box<HamtNode<Box<T>, Box<U>>>>,
    pub size: i32,
    pub hash_fn: HashFn<T>,
    pub equals_fn: EqualsFn<T>,
    _marker: std::marker::PhantomData<&'a ()>,
}

fn get_symbol(hash: u32, lvl: i32) -> u32 {
    let left = (lvl as u32) * (CHUNK_SIZE as u32);
    let left_plus_chunk = left + CHUNK_SIZE as u32;
    let right = if left_plus_chunk > 32 { 0 } else { 32 - left_plus_chunk };
    (hash << left) >> (right + left)
}

#[inline]
unsafe fn call_hash<T>(hash_fn: HashFn<T>, boxed: *mut Box<T>) -> u32 {
    (hash_fn)(&mut *(*boxed))
}

#[inline]
unsafe fn call_eq<T>(equals_fn: EqualsFn<T>, boxed: *mut Box<T>, other: &mut T) -> bool {
    (equals_fn)(&mut *(*boxed), other)
}

fn node_search<T, U>(
    node: &HamtNode<Box<T>, Box<U>>,
    hash: u32,
    lvl: i32,
    key: &mut T,
    equals_fn: EqualsFn<T>,
) -> Option<(*mut Box<T>, *mut Box<U>)> {
    match node {
        HamtNode::Leaf { key: k, value: v } => {
            if unsafe { call_eq(equals_fn, k as *const Box<T> as *mut Box<T>, key) } {
                Some((k as *const Box<T> as *mut Box<T>, v as *const Box<U> as *mut Box<U>))
            } else {
                None
            }
        }
        HamtNode::Sub { bitmap, children } => {
            let symbol = get_symbol(hash, lvl);
            let shifted = bitmap >> symbol;
            if shifted & 1 == 1 {
                let pos = (shifted >> 1).count_ones() as usize;
                node_search(&children[pos], hash, lvl + 1, key, equals_fn)
            } else {
                None
            }
        }
    }
}

fn node_insert<T: Clone, U>(
    node: &mut HamtNode<Box<T>, Box<U>>,
    hash: u32,
    lvl: i32,
    mut key: Box<T>,
    value: Box<U>,
    hash_fn: HashFn<T>,
    equals_fn: EqualsFn<T>,
) -> Result<(), (Box<T>, Box<U>)> {
    if (lvl as usize) * CHUNK_SIZE > 32 {
        return Ok(());
    }

    let is_leaf = matches!(node, HamtNode::Leaf { .. });
    if is_leaf {
        let eq = match node {
            HamtNode::Leaf { key: k, .. } => unsafe { call_eq(equals_fn, k as *mut Box<T>, &mut *key) },
            _ => unreachable!(),
        };

        if eq {
            if let HamtNode::Leaf { key: k, value: v } = node {
                let old_k = std::mem::replace(k, key);
                let old_v = std::mem::replace(v, value);
                return Err((old_k, old_v));
            }
        }

        let old = std::mem::replace(node, HamtNode::Sub { bitmap: 0, children: Vec::new() });
        if let HamtNode::Leaf { key: mut old_key, value: old_value } = old {
            let orig_hash = unsafe { call_hash(hash_fn, &mut old_key as *mut Box<T>) };
            let orig_symbol = get_symbol(orig_hash, lvl);

            *node = HamtNode::Sub {
                bitmap: 1 << orig_symbol,
                children: vec![HamtNode::Leaf { key: old_key, value: old_value }],
            };
            return node_insert(node, hash, lvl, key, value, hash_fn, equals_fn);
        }
        unreachable!()
    }

    if let HamtNode::Sub { bitmap, children } = node {
        let symbol = get_symbol(hash, lvl);
        let shifted = *bitmap >> symbol;
        if shifted & 1 == 1 {
            let pos = (shifted >> 1).count_ones() as usize;
            node_insert(&mut children[pos], hash, lvl + 1, key, value, hash_fn, equals_fn)
        } else {
            let pos = (shifted >> 1).count_ones() as usize;
            *bitmap |= 1 << symbol;
            children.insert(pos, HamtNode::Leaf { key, value });
            Ok(())
        }
    } else {
        unreachable!()
    }
}

fn node_remove<T, U>(
    node: &mut HamtNode<Box<T>, Box<U>>,
    hash: u32,
    lvl: i32,
    key: &mut T,
    equals_fn: EqualsFn<T>,
) -> Option<(Box<T>, Box<U>)> {
    let result = match node {
        HamtNode::Sub { bitmap, children } => {
            let symbol = get_symbol(hash, lvl);
            let shifted = *bitmap >> symbol;
            if shifted & 1 == 0 {
                return None;
            }
            let pos = (shifted >> 1).count_ones() as usize;

            match &children[pos] {
                HamtNode::Leaf { key: k, .. } => {
                    let eq = unsafe { call_eq(equals_fn, k as *const Box<T> as *mut Box<T>, key) };
                    if eq {
                        *bitmap &= !(1 << symbol);
                        let n = children.remove(pos);
                        match n {
                            HamtNode::Leaf { key: rk, value: rv } => Some((rk, rv)),
                            _ => unreachable!(),
                        }
                    } else {
                        None
                    }
                }
                HamtNode::Sub { .. } => {
                    node_remove(&mut children[pos], hash, lvl + 1, key, equals_fn)
                }
            }
        }
        _ => None,
    };

    if let HamtNode::Sub { bitmap, children } = node {
        if bitmap.count_ones() == 1 && children.len() == 1 && matches!(&children[0], HamtNode::Leaf { .. }) {
            let child = children.remove(0);
            *node = child;
        }
    }

    result
}

impl<'a, T: Clone, U: Clone> Hamt<'a, T, U> {
    pub fn new_hamt(hash_fn: HashFn<T>, equals_fn: EqualsFn<T>) -> Self {
        Hamt {
            root: None,
            size: 0,
            hash_fn,
            equals_fn,
            _marker: std::marker::PhantomData,
        }
    }

    pub fn hamt_size(&self) -> i32 {
        self.size
    }

    pub fn hamt_set(
        &mut self,
        key: &mut Box<T>,
        value: &mut Box<U>,
        conflict_kv: &mut KeyValue<Box<T>, Box<U>>,
    ) -> bool {
        // Clone the data into owned Boxes for storage in the HAMT.
        // The caller retains their original Boxes.
        let key_owned = Box::new((**key).clone());
        let value_owned = Box::new((**value).clone());

        if self.size == 0 {
            self.root = Some(Box::new(HamtNode::Leaf { key: key_owned, value: value_owned }));
            self.size = 1;
            return false;
        }

        let hash = (self.hash_fn)(&mut **key);

        let root = self.root.as_mut().unwrap();
        match node_insert(root, hash, 0, key_owned, value_owned, self.hash_fn, self.equals_fn) {
            Ok(()) => {
                self.size += 1;
                false
            }
            Err((old_key, old_value)) => {
                *conflict_kv.key = old_key;
                *conflict_kv.value = old_value;
                true
            }
        }
    }

    pub fn hamt_search(&self, key: &mut Box<T>) -> Option<KeyValue<'a, Box<T>, Box<U>>> {
        if self.size == 0 {
            return None;
        }
        let hash = (self.hash_fn)(&mut **key);
        let root = self.root.as_ref().unwrap();
        match node_search(root, hash, 0, &mut **key, self.equals_fn) {
            Some((k, v)) => Some(KeyValue {
                key: unsafe { &mut *k },
                value: unsafe { &mut *v },
            }),
            None => None,
        }
    }

    pub fn hamt_remove(
        &mut self,
        key: &mut Box<T>,
        removed_kv: &mut KeyValue<Box<T>, Box<U>>,
    ) -> bool {
        if self.size == 0 {
            return false;
        }

        let hash = (self.hash_fn)(&mut **key);

        let removed = if self.size == 1 {
            let eq = if let Some(root) = &self.root {
                match root.as_ref() {
                    HamtNode::Leaf { key: k, .. } => unsafe {
                        call_eq(self.equals_fn, k as *const Box<T> as *mut Box<T>, &mut **key)
                    },
                    _ => false,
                }
            } else {
                false
            };
            if eq {
                let old_root = std::mem::replace(&mut self.root, None).unwrap();
                match *old_root {
                    HamtNode::Leaf { key: rk, value: rv } => {
                        *removed_kv.key = rk;
                        *removed_kv.value = rv;
                        true
                    }
                    _ => false,
                }
            } else {
                false
            }
        } else {
            let root = self.root.as_mut().unwrap();
            match node_remove(root, hash, 0, &mut **key, self.equals_fn) {
                Some((rk, rv)) => {
                    *removed_kv.key = rk;
                    *removed_kv.value = rv;
                    true
                }
                None => false,
            }
        };

        if removed {
            self.size -= 1;
        }
        removed
    }

    pub fn hamt_destroy(
        self,
        _deallocate_fn: DeallocateFn<Box<T>>,
        _deallocate_fn_val: DeallocateFn<Box<U>>,
    ) {
    }

    pub fn hamt_print(
        &self,
        _str_fn_key: StrFn<Box<T>>,
        _str_fn_val: StrFn<Box<U>>,
    ) {
        if self.size == 0 {
            println!("{{}}");
        }
        println!("---\n");
    }
}

pub fn hamt_int_hash<T>(key: &mut T) -> u32 {
    let bytes = unsafe {
        std::slice::from_raw_parts(key as *const T as *const u8, std::mem::size_of::<T>())
    };
    let mut hash = FNV_BASE;
    for &b in bytes {
        hash = hash.wrapping_mul(FNV_PRIME);
        hash ^= b as u64;
    }
    hash as u32
}

pub fn hamt_str_hash<T>(key: &mut T) -> u32 {
    let s: &String = unsafe { &*(key as *const T as *const String) };
    let mut hash = FNV_BASE;
    for &b in s.as_bytes() {
        hash = hash.wrapping_mul(FNV_PRIME);
        hash ^= b as u64;
    }
    hash as u32
}

pub fn hamt_int_equals<T>(a: &mut T, b: &mut T) -> bool {
    let size = std::mem::size_of::<T>();
    let a_bytes = unsafe { std::slice::from_raw_parts(a as *const T as *const u8, size) };
    let b_bytes = unsafe { std::slice::from_raw_parts(b as *const T as *const u8, size) };
    a_bytes == b_bytes
}

pub fn hamt_str_equals<T>(a: &mut T, b: &mut T) -> bool {
    let sa: &String = unsafe { &*(a as *const T as *const String) };
    let sb: &String = unsafe { &*(b as *const T as *const String) };
    sa == sb
}

pub fn hamt_fnv1_hash<T>(_key: &mut T, _len: usize) {}

pub fn hamt_get_symbol(_hash: u32, _lvl: i32) {}
