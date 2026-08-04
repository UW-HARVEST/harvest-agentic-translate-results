use core::cmp::Ordering;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash as StdHash, Hasher};
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

fn compute_key_size(type_: DictType, user_size: usize) -> usize {
    match type_ {
        DictType::Char => 1,
        DictType::WChar => 4,
        DictType::I32 => 4,
        DictType::U32 => 4,
        DictType::F32 => 4,
        DictType::I64 => 8,
        DictType::U64 => 8,
        DictType::F64 => 8,
        DictType::Ptr => 8,
        DictType::Str => 8,
        DictType::Struct => (user_size + 7) & !7,
    }
}

fn align_val_size(size: usize) -> usize {
    (size + 7) & !7
}

fn compute_hash_raw(type_: DictType, key_size: usize, hash_fn: Option<DictHash>, key: &[u8]) -> u64 {
    if let Some(h) = hash_fn {
        return h(key);
    }
    match type_ {
        DictType::Str => {
            let mut code: u64 = 0;
            for &b in key {
                code = (code.wrapping_mul(HASH_BASE).wrapping_add(b as u64)) % HASH_MOD;
            }
            code
        }
        DictType::Struct => {
            let mut code: u64 = 0;
            for &b in &key[..key_size] {
                code = (code.wrapping_mul(HASH_BASE).wrapping_add(b as u64)) % HASH_MOD;
            }
            code
        }
        DictType::Char => key[0] as u64,
        DictType::WChar => {
            let mut buf = [0u8; 4];
            buf[..key.len().min(4)].copy_from_slice(&key[..key.len().min(4)]);
            i32::from_ne_bytes(buf) as u64
        }
        DictType::I32 => {
            let mut buf = [0u8; 4];
            buf.copy_from_slice(&key[..4]);
            i32::from_ne_bytes(buf) as u64
        }
        DictType::U32 => {
            let mut buf = [0u8; 4];
            buf.copy_from_slice(&key[..4]);
            u32::from_ne_bytes(buf) as u64
        }
        DictType::F32 => {
            let mut buf = [0u8; 4];
            buf.copy_from_slice(&key[..4]);
            f32::from_ne_bytes(buf) as u64
        }
        DictType::I64 => {
            let mut buf = [0u8; 8];
            buf.copy_from_slice(&key[..8]);
            i64::from_ne_bytes(buf) as u64
        }
        DictType::U64 | DictType::Ptr => {
            let mut buf = [0u8; 8];
            buf.copy_from_slice(&key[..8]);
            u64::from_ne_bytes(buf)
        }
        DictType::F64 => {
            let mut buf = [0u8; 8];
            buf.copy_from_slice(&key[..8]);
            f64::from_ne_bytes(buf) as u64
        }
    }
}

fn keys_equal_raw(type_: DictType, key_size: usize, cmpr: Option<DictCmpr>, a: &[u8], b: &[u8]) -> bool {
    if let Some(c) = cmpr {
        return c(a, b) == 0;
    }
    match type_ {
        DictType::Str => a == b,
        _ => a[..key_size] == b[..key_size],
    }
}

fn prepare_key(type_: DictType, key_size: usize, key_data: &[u8]) -> Vec<u8> {
    match type_ {
        DictType::Str => key_data.to_vec(),
        _ => {
            let mut key = vec![0u8; key_size];
            let len = key_data.len().min(key_size);
            key[..len].copy_from_slice(&key_data[..len]);
            key
        }
    }
}

/// Create a dictionary using detailed arguments. Matches C's dict_create().
pub fn dict_create(args: DictArgs) -> Dict {
    let key_size = compute_key_size(args.key.type_, args.key.size);
    let val_size = align_val_size(args.val.size);
    Dict {
        key: DictKeyAttr { type_: args.key.type_, size: key_size, copy: args.key.copy, free: args.key.free, hash: args.key.hash, cmpr: args.key.cmpr },
        val: DictValAttr { size: val_size, free: args.val.free },
        alloc: args.alloc,
        mod_: DEFAULT_MOD,
        buckets: (0..DEFAULT_MOD).map(|_| DictBucket { elements: Vec::new() }).collect(),
        key_temp: vec![0u8; key_size],
        keys_dump: Vec::new(),
        count: 0,
    }
}
/// Create a dictionary with derived arguments. Matches C's dict_new().
pub fn dict_new(key_type: DictType, key_size: usize, val_size: usize) -> Dict {
    dict_create(DictArgs {
        key: DictKeyAttr { type_: key_type, size: key_size, copy: None, free: None, hash: None, cmpr: None },
        val: DictValAttr { size: val_size, free: None },
        alloc: DictAlloc { malloc: None, free: None },
    })
}
/// Destroy a dictionary. Matches C's dict_destroy().
pub fn dict_destroy(dict: &mut Dict) {
    for bucket in &mut dict.buckets {
        for elem in &mut bucket.elements {
            if let Some(kf) = dict.key.free { kf(&mut elem.key); }
            if dict.val.size != 0 {
                if let Some(vf) = dict.val.free { vf(&mut elem.val); }
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
    let type_ = dict.key.type_;
    let key_size = dict.key.size;
    let cmpr = dict.key.cmpr;
    let hash_fn = dict.key.hash;
    let val_size = dict.val.size;
    let mod_ = dict.mod_;

    let key = prepare_key(type_, key_size, key_data);
    let code = compute_hash_raw(type_, key_size, hash_fn, &key);
    let index = (code as usize) % mod_;

    // Search for existing element
    let found = dict.buckets[index].elements.iter().position(|elem| {
        elem.code == code && keys_equal_raw(type_, key_size, cmpr, &elem.key, &key)
    });

    if let Some(pos) = found {
        return Some(&mut dict.buckets[index].elements[pos].val);
    }

    // Not found - insert
    let bucket_len = dict.buckets[index].elements.len();
    dict.buckets[index].elements.push(DictElem {
        code,
        key,
        val: vec![0u8; val_size],
    });

    // Check if reshape needed (C: if bucket.size++ > dict->mod)
    if bucket_len > mod_ {
        dict_reshape(dict, 1);
        // Find element after reshape
        let new_key = prepare_key(type_, key_size, key_data);
        let new_index = (code as usize) % dict.mod_;
        let pos = dict.buckets[new_index].elements.iter().position(|elem| {
            elem.code == code && keys_equal_raw(type_, key_size, cmpr, &elem.key, &new_key)
        });
        if let Some(p) = pos {
            return Some(&mut dict.buckets[new_index].elements[p].val);
        }
        return None;
    }

    let last = dict.buckets[index].elements.last_mut().unwrap();
    Some(&mut last.val)
}
/// Remove a value from the dictionary.
pub fn dict_remove(dict: &mut Dict, key_data: &[u8]) -> bool {
    let type_ = dict.key.type_;
    let key_size = dict.key.size;
    let cmpr = dict.key.cmpr;
    let hash_fn = dict.key.hash;

    let key = prepare_key(type_, key_size, key_data);
    let code = compute_hash_raw(type_, key_size, hash_fn, &key);
    let index = (code as usize) % dict.mod_;

    let bucket = &mut dict.buckets[index];
    let pos = bucket.elements.iter().position(|elem| {
        elem.code == code && keys_equal_raw(type_, key_size, cmpr, &elem.key, &key)
    });

    if let Some(i) = pos {
        let mut removed = bucket.elements.remove(i);
        if let Some(kf) = dict.key.free { kf(&mut removed.key); }
        if let Some(vf) = dict.val.free { vf(&mut removed.val); }
        return true;
    }
    false
}
/// Check if a key exists in the dictionary.
pub fn dict_has(dict: &Dict, key_data: &[u8]) -> bool {
    let key = prepare_key(dict.key.type_, dict.key.size, key_data);
    let code = compute_hash_raw(dict.key.type_, dict.key.size, dict.key.hash, &key);
    let index = (code as usize) % dict.mod_;

    dict.buckets[index].elements.iter().any(|elem| {
        elem.code == code && keys_equal_raw(dict.key.type_, dict.key.size, dict.key.cmpr, &elem.key, &key)
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

    let mut arr = Vec::with_capacity(dict.key.size * (*size));
    for bucket in &dict.buckets {
        for elem in &bucket.elements {
            if dict.key.type_ == DictType::Str {
                let mut slot = vec![0u8; dict.key.size];
                let len = elem.key.len().min(dict.key.size);
                slot[..len].copy_from_slice(&elem.key[..len]);
                arr.extend_from_slice(&slot);
            } else {
                arr.extend_from_slice(&elem.key[..dict.key.size]);
            }
        }
    }

    Some(Box::leak(arr.into_boxed_slice()))
}
/// Serialize a dictionary into a contiguous Vec<u8>.
pub fn dict_serialize(dict: &Dict, bytes: &mut usize) -> Option<Vec<u8>> {
    let count = dict_len(dict) as u32;
    let header: [u32; 3] = [dict.key.size as u32, dict.val.size as u32, count];

    if dict.key.type_ == DictType::Str {
        let elem_size = 4 + dict.val.size;
        let mut strlen_table: Vec<u32> = Vec::with_capacity(count as usize);
        let mut total_str_bytes: usize = 0;
        for bucket in &dict.buckets {
            for elem in &bucket.elements {
                let slen = elem.key.len() as u32;
                strlen_table.push(slen);
                total_str_bytes += slen as usize;
            }
        }

        *bytes = 12 + (count as usize) * elem_size + total_str_bytes;
        let mut data = Vec::with_capacity(*bytes);
        for h in &header { data.extend_from_slice(&h.to_ne_bytes()); }

        let mut str_data: Vec<u8> = Vec::with_capacity(total_str_bytes);
        let mut idx = 0;
        for bucket in &dict.buckets {
            for elem in &bucket.elements {
                data.extend_from_slice(&strlen_table[idx].to_ne_bytes());
                data.extend_from_slice(&elem.val[..dict.val.size]);
                str_data.extend_from_slice(&elem.key);
                idx += 1;
            }
        }
        data.extend_from_slice(&str_data);
        Some(data)
    } else {
        let elem_size = dict.key.size + dict.val.size;
        *bytes = 12 + (count as usize) * elem_size;
        let mut data = Vec::with_capacity(*bytes);
        for h in &header { data.extend_from_slice(&h.to_ne_bytes()); }
        for bucket in &dict.buckets {
            for elem in &bucket.elements {
                data.extend_from_slice(&elem.key[..dict.key.size]);
                data.extend_from_slice(&elem.val[..dict.val.size]);
            }
        }
        Some(data)
    }
}
/// Deserialize a dictionary from a slice.
pub fn dict_deserialize(args: DictArgs, data: &[u8]) -> Dict {
    // Skip text size prefix if present (e.g. "6012\n" written by serialize+file write)
    let data = if let Some(pos) = data.iter().position(|&b| b == b'\n') {
        if data[..pos].iter().all(|&b| b.is_ascii_digit()) {
            &data[pos + 1..]
        } else {
            data
        }
    } else {
        data
    };
    let key_size_stored = u32::from_ne_bytes([data[0], data[1], data[2], data[3]]) as usize;
    let val_size_stored = u32::from_ne_bytes([data[4], data[5], data[6], data[7]]) as usize;
    let count = u32::from_ne_bytes([data[8], data[9], data[10], data[11]]) as usize;

    let key_size = compute_key_size(args.key.type_, args.key.size);
    let val_size = align_val_size(args.val.size);

    let type_ = args.key.type_;
    let hash_fn = args.key.hash;

    let mut dict = Dict {
        key: DictKeyAttr { type_: args.key.type_, size: key_size, copy: args.key.copy, free: args.key.free, hash: args.key.hash, cmpr: args.key.cmpr },
        val: DictValAttr { size: val_size, free: args.val.free },
        alloc: args.alloc,
        mod_: DEFAULT_MOD,
        buckets: (0..DEFAULT_MOD).map(|_| DictBucket { elements: Vec::new() }).collect(),
        key_temp: vec![0u8; key_size],
        keys_dump: Vec::new(),
        count: 0,
    };

    // C returns NULL on key/val size mismatch; return empty dict
    if key_size != key_size_stored || val_size != val_size_stored {
        return dict;
    }

    let mut ptr = 12;

    if type_ == DictType::Str {
        let elem_size = 4 + val_size_stored;
        let str_start = ptr + count * elem_size;
        let mut str_ptr = str_start;

        for _ in 0..count {
            let slen = u32::from_ne_bytes([data[ptr], data[ptr+1], data[ptr+2], data[ptr+3]]) as usize;
            ptr += 4;
            let mut val = vec![0u8; val_size];
            let copy_len = val_size_stored.min(val_size);
            val[..copy_len].copy_from_slice(&data[ptr..ptr + copy_len]);
            ptr += val_size_stored;
            let key_bytes = data[str_ptr..str_ptr + slen].to_vec();
            str_ptr += slen;

            let code = compute_hash_raw(type_, key_size, hash_fn, &key_bytes);
            let index = (code as usize) % dict.mod_;
            dict.buckets[index].elements.push(DictElem { code, key: key_bytes, val });
        }
    } else {
        for _ in 0..count {
            let mut key = vec![0u8; key_size];
            let klen = key_size_stored.min(key_size);
            key[..klen].copy_from_slice(&data[ptr..ptr + klen]);
            ptr += key_size_stored;

            let mut val = vec![0u8; val_size];
            let copy_len = val_size_stored.min(val_size);
            val[..copy_len].copy_from_slice(&data[ptr..ptr + copy_len]);
            ptr += val_size_stored;

            let code = compute_hash_raw(type_, key_size, hash_fn, &key);
            let index = (code as usize) % dict.mod_;
            dict.buckets[index].elements.push(DictElem { code, key, val });
        }
    }

    let max = dict.buckets.iter().map(|b| b.elements.len()).max().unwrap_or(0);
    if max > DEFAULT_MOD {
        dict_reshape(&mut dict, max / DEFAULT_MOD);
    }

    dict
}
/// Convenience function to create a dictionary using inline arguments.
pub fn dict_create_args(args: DictArgs) -> Dict {
    dict_create(args)
}
/// The original dict_key_equals.
pub fn dict_key_equals(dict: &Dict, a: &[u8], b: &[u8]) -> bool {
    keys_equal_raw(dict.key.type_, dict.key.size, dict.key.cmpr, a, b)
}
/// Not used in this design, but signature is kept.
pub fn dict_delete_node(_list: &mut DictBucket, _curr: &mut DictElem) {
}
/// The original dict_free_val.
pub fn dict_free_val(dict: &Dict, val: &mut [u8]) {
    if let Some(f) = dict.val.free { f(val); }
}
/// Not used in pure Rust version, matching signature only.
pub fn dict_get_key(_dict: &Dict) -> Option<&mut [u8]> {
    None
}
/// Internal function to reshape the dictionary.
pub fn dict_reshape(dict: &mut Dict, step: usize) -> bool {
    let new_size = dict.mod_ * step * DEFAULT_STEP;
    let mut new_buckets: Vec<DictBucket> = (0..new_size).map(|_| DictBucket { elements: Vec::new() }).collect();
    for bucket in &mut dict.buckets {
        for elem in bucket.elements.drain(..) {
            let index = (elem.code as usize) % new_size;
            new_buckets[index].elements.push(elem);
        }
    }
    dict.mod_ = new_size;
    dict.buckets = new_buckets;
    true
}
/// Internal function to free a node.
pub fn dict_free_node(_dict: &Dict, _node: &mut DictElem) {
}
/// Internal function to free a dictionary key.
pub fn dict_free_key(dict: &Dict, key: &mut [u8]) {
    if let Some(f) = dict.key.free { f(key); }
}
/// The original dict_get_hash.
pub fn dict_get_hash(dict: &Dict, key: &[u8]) -> u64 {
    compute_hash_raw(dict.key.type_, dict.key.size, dict.key.hash, key)
}
