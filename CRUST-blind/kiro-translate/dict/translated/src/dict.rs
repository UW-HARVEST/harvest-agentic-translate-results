#[allow(unused_imports)]
use core::cmp::Ordering;
#[allow(unused_imports)]
use std::collections::hash_map::DefaultHasher;
#[allow(unused_imports)]
use std::hash::{Hash as StdHash, Hasher};
#[allow(unused_imports)]
use std::sync::Mutex;
/// A collection of constants matching the original C #defines.
pub const HASH_MOD: u64 = 1000000007;
pub const HASH_BASE: u64 = 256;
pub const DEFAULT_STEP: usize = 2;
pub const DEFAULT_MOD: usize = 8;
/// Enum corresponding to the C enum `dict_type_t`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DictType {
Char,
WChar,
I32,
U32,
F32,
I64,
U64,
F64,
Ptr,
Str,
Struct,
}
/// A safe function pointer type for deep-copying values.
pub type DictDeepCopy = fn(dest: &mut [u8], src: &[u8]);
/// A safe function pointer type for destructors.
pub type DictDestructor = fn(data: &mut [u8]);
/// A safe function pointer type for comparisons.
pub type DictCmpr = fn(a: &[u8], b: &[u8]) -> i32;
/// A safe function pointer type for hashing.
pub type DictHash = fn(data: &[u8]) -> u64;
/// A safe function pointer type for memory allocation (unused in safe Rust).
pub type DictMalloc = fn(size: usize) -> Vec<u8>;
/// A safe function pointer type for freeing allocated memory (unused in safe Rust).
pub type DictFree = fn(_: Vec<u8>);
/// Corresponds to `dict_alloc_t` in C.
#[derive(Clone)]
pub struct DictAlloc {
pub malloc: Option<DictMalloc>,
pub free: Option<DictFree>,
}
/// Corresponds to `dict_key_attr_t` in C.
#[derive(Clone)]
pub struct DictKeyAttr {
pub type_: DictType,
pub size: usize,
pub copy: Option<DictDeepCopy>,
pub free: Option<DictDestructor>,
pub hash: Option<DictHash>,
pub cmpr: Option<DictCmpr>,
}
/// Corresponds to `dict_val_attr_t` in C.
#[derive(Clone)]
pub struct DictValAttr {
pub size: usize,
pub free: Option<DictDestructor>,
}
/// Corresponds to `dict_args_t` in C.
#[derive(Clone)]
pub struct DictArgs {
pub key: DictKeyAttr,
pub val: DictValAttr,
pub alloc: DictAlloc,
}
/// Each element in the dictionary. Mirrors the struct dict_elem in C.
/// We store the key bytes in `key` and the value bytes in `val`.
#[derive(Clone)]
pub struct DictElem {
pub code: u64,    // The hash code
pub key: Vec<u8>, // Owned bytes for the key
pub val: Vec<u8>, // Owned bytes for the value
}
/// A bucket (list) in the dictionary. Mirrors dict_list_t in C, but
/// instead of a linked list, we store a Vec for safe iteration/removal.
#[derive(Clone)]
pub struct DictBucket {
pub elements: Vec<DictElem>,
}
/// Corresponds to `struct dict` in C.
pub struct Dict {
pub key: DictKeyAttr,
pub val: DictValAttr,
pub alloc: DictAlloc,
pub mod_: usize,              // Number of buckets
pub buckets: Vec<DictBucket>, // The buckets array
pub key_temp: Vec<u8>,        // Temporary buffer for constructing a key
pub keys_dump: Vec<u8>,       // Unused here; effectively replaced by a safe approach
pub count: usize,             // Number of elements total
}

// ── helpers ──────────────────────────────────────────────────────────

fn key_size_for_type(t: DictType) -> usize {
    match t {
        DictType::Char   => 1,
        DictType::WChar  => 4,
        DictType::I32    => 4,
        DictType::U32    => 4,
        DictType::F32    => 4,
        DictType::I64    => 8,
        DictType::U64    => 8,
        DictType::F64    => 8,
        DictType::Ptr    => 8,
        DictType::Str    => 8, // pointer-sized
        DictType::Struct => 0, // caller must provide
    }
}

fn align_to_ptr(sz: usize) -> usize {
    let a = std::mem::size_of::<usize>();
    (sz + (a - 1)) & !(a - 1)
}

fn compute_key_size(t: DictType, user_size: usize) -> usize {
    match t {
        DictType::Struct => align_to_ptr(user_size),
        _ => key_size_for_type(t),
    }
}

fn compute_val_size(user_size: usize) -> usize {
    align_to_ptr(user_size)
}

/// Prepare key bytes: apply copy callback or handle Str type (clone the string bytes).
fn prepare_key(dict: &Dict, key_data: &[u8]) -> Vec<u8> {
    if let Some(copy_fn) = dict.key.copy {
        let mut buf = vec![0u8; dict.key.size];
        copy_fn(&mut buf, key_data);
        buf
    } else if dict.key.type_ == DictType::Str {
        // key_data is the raw string bytes; we store them as-is
        key_data.to_vec()
    } else {
        // For fixed-size types / struct: just copy the bytes (up to key.size)
        let mut buf = vec![0u8; dict.key.size];
        let len = key_data.len().min(dict.key.size);
        buf[..len].copy_from_slice(&key_data[..len]);
        buf
    }
}

/// Compute hash code for a key.
fn compute_hash(dict: &Dict, key: &[u8]) -> u64 {
    if let Some(hash_fn) = dict.key.hash {
        return hash_fn(key);
    }
    match dict.key.type_ {
        DictType::Str => {
            // key is raw string bytes; polynomial hash
            let mut code: u64 = 0;
            for &b in key.iter() {
                code = (code.wrapping_mul(HASH_BASE).wrapping_add(b as u64)) % HASH_MOD;
            }
            code
        }
        DictType::Struct => {
            let mut code: u64 = 0;
            for &b in key.iter() {
                code = (code.wrapping_mul(HASH_BASE).wrapping_add(b as u64)) % HASH_MOD;
            }
            code
        }
        DictType::Char => {
            if key.is_empty() { 0 } else { key[0] as u64 }
        }
        DictType::WChar | DictType::I32 | DictType::U32 | DictType::F32 => {
            let mut arr = [0u8; 4];
            let len = key.len().min(4);
            arr[..len].copy_from_slice(&key[..len]);
            u32::from_ne_bytes(arr) as u64
        }
        DictType::I64 | DictType::U64 | DictType::F64 | DictType::Ptr => {
            let mut arr = [0u8; 8];
            let len = key.len().min(8);
            arr[..len].copy_from_slice(&key[..len]);
            u64::from_ne_bytes(arr)
        }
    }
}

/// Compare two keys for equality.
fn keys_equal(dict: &Dict, a: &[u8], b: &[u8]) -> bool {
    if let Some(cmpr) = dict.key.cmpr {
        return cmpr(a, b) == 0;
    }
    match dict.key.type_ {
        DictType::Str => a == b, // both are raw string bytes
        _ => a == b,
    }
}

/// Free key via destructor if applicable.
fn free_key(dict: &Dict, key: &mut Vec<u8>) {
    if dict.key.copy.is_some() {
        if let Some(free_fn) = dict.key.free {
            free_fn(key);
        }
    }
    // For Str without custom copy, nothing special needed in Rust (Vec drops)
}

/// Free val via destructor if applicable.
fn free_val_bytes(dict: &Dict, val: &mut Vec<u8>) {
    if let Some(free_fn) = dict.val.free {
        free_fn(val);
    }
}

// ── public API ───────────────────────────────────────────────────────

/// Create a dictionary using detailed arguments. Matches C's dict_create().
pub fn dict_create(args: DictArgs) -> Dict {
    let key_size = compute_key_size(args.key.type_, args.key.size);
    let val_size = compute_val_size(args.val.size);

    let mut buckets = Vec::with_capacity(DEFAULT_MOD);
    for _ in 0..DEFAULT_MOD {
        buckets.push(DictBucket { elements: Vec::new() });
    }

    Dict {
        key: DictKeyAttr { size: key_size, ..args.key },
        val: DictValAttr { size: val_size, ..args.val },
        alloc: args.alloc,
        mod_: DEFAULT_MOD,
        buckets,
        key_temp: vec![0u8; key_size],
        keys_dump: Vec::new(),
        count: 0,
    }
}

/// Create a dictionary with derived arguments. Matches C's dict_new().
pub fn dict_new(key_type: DictType, key_size: usize, val_size: usize) -> Dict {
    dict_create(DictArgs {
        key: DictKeyAttr {
            type_: key_type,
            size: key_size,
            copy: None,
            free: None,
            hash: None,
            cmpr: None,
        },
        val: DictValAttr { size: val_size, free: None },
        alloc: DictAlloc { malloc: None, free: None },
    })
}

/// Destroy a dictionary. Matches C's dict_destroy().
pub fn dict_destroy(dict: &mut Dict) {
    let key_copy = dict.key.copy;
    let key_free = dict.key.free;
    let val_free = dict.val.free;
    let val_size = dict.val.size;
    for bucket in dict.buckets.iter_mut() {
        for elem in bucket.elements.iter_mut() {
            if key_copy.is_some() {
                if let Some(kf) = key_free {
                    kf(&mut elem.key);
                }
            }
            if val_size != 0 {
                if let Some(vf) = val_free {
                    vf(&mut elem.val);
                }
            }
        }
        bucket.elements.clear();
    }
    dict.buckets.clear();
    dict.mod_ = 0;
    dict.count = 0;
}

/// Retrieve or create a value from the dictionary.
pub fn dict_get<'dict>(dict: &'dict mut Dict, key_data: &[u8]) -> Option<&'dict mut [u8]> {
    let key = prepare_key(dict, key_data);
    let code = compute_hash(dict, &key);
    let index = (code as usize) % dict.mod_;

    // Search existing
    let found = dict.buckets[index].elements.iter().position(|e| {
        e.code == code && keys_equal(dict, &e.key, &key)
    });

    if let Some(pos) = found {
        let val_size = dict.val.size;
        return Some(&mut dict.buckets[index].elements[pos].val[..val_size]);
    }

    // Insert new
    let val_size = dict.val.size;
    let bucket_size_before = dict.buckets[index].elements.len();
    dict.buckets[index].elements.push(DictElem {
        code,
        key,
        val: vec![0u8; val_size],
    });
    dict.count += 1;

    // Reshape if needed
    if bucket_size_before > dict.mod_ {
        dict_reshape(dict, 1);
        // After reshape, find the element again
        let new_index = (code as usize) % dict.mod_;
        let pos = dict.buckets[new_index].elements.iter().position(|e| e.code == code).unwrap();
        return Some(&mut dict.buckets[new_index].elements[pos].val[..val_size]);
    }

    let len = dict.buckets[index].elements.len();
    Some(&mut dict.buckets[index].elements[len - 1].val[..val_size])
}

/// Remove a value from the dictionary. Returns true if found and removed.
pub fn dict_remove(dict: &mut Dict, key_data: &[u8]) -> bool {
    let key = prepare_key(dict, key_data);
    let code = compute_hash(dict, &key);
    let index = (code as usize) % dict.mod_;

    let pos = dict.buckets[index].elements.iter().position(|e| {
        e.code == code && keys_equal(dict, &e.key, &key)
    });

    if let Some(pos) = pos {
        let mut elem = dict.buckets[index].elements.remove(pos);
        free_key(dict, &mut elem.key);
        free_val_bytes(dict, &mut elem.val);
        dict.count -= 1;
        true
    } else {
        false
    }
}

/// Check if a key exists in the dictionary.
pub fn dict_has(dict: &Dict, key_data: &[u8]) -> bool {
    let key = prepare_key(dict, key_data);
    let code = compute_hash(dict, &key);
    let index = (code as usize) % dict.mod_;

    dict.buckets[index].elements.iter().any(|e| {
        e.code == code && keys_equal(dict, &e.key, &key)
    })
}

/// Return the number of elements in the dictionary.
pub fn dict_len(dict: &Dict) -> usize {
    dict.buckets.iter().map(|b| b.elements.len()).sum()
}

/// Return a snapshot of all keys.
pub fn dict_key(dict: &Dict, size: &mut usize) -> Option<&'static [u8]> {
    *size = dict_len(dict);
    if *size == 0 {
        return None;
    }

    let key_size = dict.key.size;
    let mut arr = vec![0u8; key_size * (*size)];
    let mut idx = 0;
    for bucket in &dict.buckets {
        for elem in &bucket.elements {
            let start = key_size * idx;
            let len = elem.key.len().min(key_size);
            arr[start..start + len].copy_from_slice(&elem.key[..len]);
            idx += 1;
            if idx == *size {
                let leaked = arr.leak();
                return Some(leaked);
            }
        }
    }
    let leaked = arr.leak();
    Some(leaked)
}

/// Serialize a dictionary into a contiguous Vec<u8>.
pub fn dict_serialize(dict: &Dict, bytes: &mut usize) -> Option<Vec<u8>> {
    let size = dict_len(dict) as u32;
    let key_val_size: [u32; 3] = [dict.key.size as u32, dict.val.size as u32, size];

    let is_str = dict.key.type_ == DictType::Str;
    let elem_size = if is_str {
        4 + dict.val.size // u32 strlen + val
    } else {
        dict.key.size + dict.val.size
    };

    // Calculate total bytes
    let mut total = 12 + (size as usize) * elem_size; // 3 * u32 header

    // For string keys, add the actual string data lengths
    let mut strlen_table: Vec<u32> = Vec::new();
    if is_str {
        for bucket in &dict.buckets {
            for elem in &bucket.elements {
                let slen = elem.key.len() as u32;
                strlen_table.push(slen);
                total += slen as usize;
            }
        }
    }

    let mut data = vec![0u8; total];
    let mut ptr = 0;

    // Header
    for &v in &key_val_size {
        data[ptr..ptr + 4].copy_from_slice(&v.to_ne_bytes());
        ptr += 4;
    }

    if is_str {
        // First pass: write strlen + val entries
        let str_data_offset = ptr + (size as usize) * elem_size;
        let mut str_ptr = str_data_offset;
        let mut si = 0;
        for bucket in &dict.buckets {
            for elem in &bucket.elements {
                let slen = strlen_table[si];
                data[ptr..ptr + 4].copy_from_slice(&slen.to_ne_bytes());
                ptr += 4;
                let vlen = dict.val.size.min(elem.val.len());
                data[ptr..ptr + vlen].copy_from_slice(&elem.val[..vlen]);
                ptr += dict.val.size;
                let klen = elem.key.len();
                data[str_ptr..str_ptr + klen].copy_from_slice(&elem.key[..klen]);
                str_ptr += klen;
                si += 1;
            }
        }
    } else {
        for bucket in &dict.buckets {
            for elem in &bucket.elements {
                let klen = dict.key.size.min(elem.key.len());
                data[ptr..ptr + klen].copy_from_slice(&elem.key[..klen]);
                ptr += dict.key.size;
                let vlen = dict.val.size.min(elem.val.len());
                data[ptr..ptr + vlen].copy_from_slice(&elem.val[..vlen]);
                ptr += dict.val.size;
            }
        }
    }

    *bytes = total;
    Some(data)
}

/// Deserialize a dictionary from a slice.
pub fn dict_deserialize(args: DictArgs, data: &[u8]) -> Dict {
    let mut ptr = 0;

    let mut key_val_size = [0u32; 3];
    for i in 0..3 {
        key_val_size[i] = u32::from_ne_bytes(data[ptr..ptr + 4].try_into().unwrap());
        ptr += 4;
    }

    let _key_size = compute_key_size(args.key.type_, args.key.size);
    let _val_size = compute_val_size(args.val.size);

    let mut dict = dict_create(args);

    let count = key_val_size[2] as usize;
    let is_str = dict.key.type_ == DictType::Str;
    let elem_size = if is_str {
        4 + dict.val.size
    } else {
        dict.key.size + dict.val.size
    };

    if is_str {
        let str_data_start = ptr + count * elem_size;
        let mut str_ptr = str_data_start;
        for _ in 0..count {
            let slen = u32::from_ne_bytes(data[ptr..ptr + 4].try_into().unwrap()) as usize;
            ptr += 4;
            let mut val = vec![0u8; dict.val.size];
            let vlen = dict.val.size;
            val[..vlen].copy_from_slice(&data[ptr..ptr + vlen]);
            ptr += dict.val.size;
            let key = data[str_ptr..str_ptr + slen].to_vec();
            str_ptr += slen;

            let code = compute_hash(&dict, &key);
            let index = (code as usize) % dict.mod_;
            dict.buckets[index].elements.push(DictElem { code, key, val });
            dict.count += 1;
        }
    } else {
        for _ in 0..count {
            let mut key = vec![0u8; dict.key.size];
            key.copy_from_slice(&data[ptr..ptr + dict.key.size]);
            ptr += dict.key.size;
            let mut val = vec![0u8; dict.val.size];
            val.copy_from_slice(&data[ptr..ptr + dict.val.size]);
            ptr += dict.val.size;

            let code = compute_hash(&dict, &key);
            let index = (code as usize) % dict.mod_;
            dict.buckets[index].elements.push(DictElem { code, key, val });
            dict.count += 1;
        }
    }

    // Reshape if needed (same logic as C)
    let max = dict.buckets.iter().map(|b| b.elements.len()).max().unwrap_or(0);
    if max > DEFAULT_MOD {
        let step = max / DEFAULT_MOD;
        dict_reshape(&mut dict, step);
    }

    dict
}

/// Convenience function to create a dictionary using inline arguments.
pub fn dict_create_args(args: DictArgs) -> Dict {
    dict_create(args)
}

/// Key equality check.
pub fn dict_key_equals(dict: &Dict, a: &[u8], b: &[u8]) -> bool {
    keys_equal(dict, a, b)
}

/// Not used in this safe design.
pub fn dict_delete_node(_list: &mut DictBucket, _curr: &mut DictElem) {
// no-op in this safe design
}

/// Free val via destructor.
pub fn dict_free_val(dict: &Dict, val: &mut [u8]) {
    if let Some(free_fn) = dict.val.free {
        free_fn(val);
    }
}

/// Not used in pure Rust version, matching signature only.
pub fn dict_get_key(_dict: &Dict) -> Option<&mut [u8]> {
    None
}

/// Reshape the dictionary.
pub fn dict_reshape(dict: &mut Dict, step: usize) -> bool {
    let new_size = dict.mod_ * step * DEFAULT_STEP;
    let mut new_buckets = Vec::with_capacity(new_size);
    for _ in 0..new_size {
        new_buckets.push(DictBucket { elements: Vec::new() });
    }

    let old_buckets = std::mem::replace(&mut dict.buckets, new_buckets);
    dict.mod_ = new_size;

    for bucket in old_buckets {
        for elem in bucket.elements {
            let index = (elem.code as usize) % new_size;
            dict.buckets[index].elements.push(elem);
        }
    }
    true
}

/// Free a node (no-op in safe Rust).
pub fn dict_free_node(_dict: &Dict, _node: &mut DictElem) {
    // no-op
}

/// Free a key via destructor.
pub fn dict_free_key(dict: &Dict, key: &mut [u8]) {
    if dict.key.copy.is_some() {
        if let Some(free_fn) = dict.key.free {
            free_fn(key);
        }
    }
}

/// Compute hash for a key.
pub fn dict_get_hash(dict: &Dict, key: &[u8]) -> u64 {
    compute_hash(dict, key)
}
