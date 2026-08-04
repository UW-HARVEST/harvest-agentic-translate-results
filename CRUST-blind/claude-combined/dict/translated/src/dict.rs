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

#[allow(dead_code)]
fn ptr_align(size: usize) -> usize {
    let align = std::mem::size_of::<usize>();
    (size + (align - 1)) & !(align - 1)
}

#[allow(dead_code)]
fn key_size_for(type_: DictType, struct_size: usize) -> usize {
    match type_ {
        DictType::Char => 1,
        DictType::WChar => 4, // wchar_t on Linux is 4 bytes
        DictType::I32 => 4,
        DictType::U32 => 4,
        DictType::F32 => 4,
        DictType::I64 => 8,
        DictType::U64 => 8,
        DictType::F64 => 8,
        DictType::Ptr => std::mem::size_of::<usize>(),
        DictType::Str => std::mem::size_of::<usize>(),
        DictType::Struct => ptr_align(struct_size),
    }
}

/// Internal hash computation that mirrors C's dict_get_hash.
fn compute_hash(dict: &Dict, key_bytes: &[u8]) -> u64 {
    if let Some(hash_fn) = dict.key.hash {
        return hash_fn(key_bytes);
    }
    match dict.key.type_ {
        DictType::Char => {
            // signed char as i64 then u64 (but Rust's b as i8 as u64 sign extends)
            (key_bytes[0] as i8 as i64) as u64
        }
        DictType::WChar => {
            let v = i32::from_ne_bytes([key_bytes[0], key_bytes[1], key_bytes[2], key_bytes[3]]);
            v as i64 as u64
        }
        DictType::I32 => {
            let v = i32::from_ne_bytes([key_bytes[0], key_bytes[1], key_bytes[2], key_bytes[3]]);
            v as i64 as u64
        }
        DictType::U32 => {
            let v = u32::from_ne_bytes([key_bytes[0], key_bytes[1], key_bytes[2], key_bytes[3]]);
            v as u64
        }
        DictType::F32 => {
            let v = f32::from_ne_bytes([key_bytes[0], key_bytes[1], key_bytes[2], key_bytes[3]]);
            // C does: code = *(float*) key  => float-to-uint64_t. On x86_64 GCC, this uses
            // signed conversion (cvttss2si), so negative floats wrap. Replicate by going
            // through i64 first.
            (v as i64) as u64
        }
        DictType::I64 => {
            let bytes: [u8; 8] = key_bytes[0..8].try_into().unwrap();
            let v = i64::from_ne_bytes(bytes);
            v as u64
        }
        DictType::U64 => {
            let bytes: [u8; 8] = key_bytes[0..8].try_into().unwrap();
            u64::from_ne_bytes(bytes)
        }
        DictType::F64 => {
            let bytes: [u8; 8] = key_bytes[0..8].try_into().unwrap();
            let v = f64::from_ne_bytes(bytes);
            // Match C semantics: float-to-uint64 via signed conversion
            (v as i64) as u64
        }
        DictType::Ptr => {
            let bytes: [u8; 8] = key_bytes[0..8].try_into().unwrap();
            u64::from_ne_bytes(bytes)
        }
        DictType::Str => {
            // For Str, key_bytes contains the string bytes (no null terminator)
            let mut code: u64 = 0;
            for &b in key_bytes {
                let signed = (b as i8) as i64 as u64;
                code = code.wrapping_mul(HASH_BASE).wrapping_add(signed) % HASH_MOD;
            }
            code
        }
        DictType::Struct => {
            let mut code: u64 = 0;
            // Iterate over the bytes in the (padded) struct slot
            let length = dict.key.size;
            for i in 0..length {
                let b = if i < key_bytes.len() { key_bytes[i] } else { 0 };
                let signed = (b as i8) as i64 as u64;
                code = code.wrapping_mul(HASH_BASE).wrapping_add(signed) % HASH_MOD;
            }
            code
        }
    }
}

/// Compare two key byte slices.
fn keys_equal(dict: &Dict, a: &[u8], b: &[u8]) -> bool {
    if let Some(cmpr) = dict.key.cmpr {
        return cmpr(a, b) == 0;
    }
    match dict.key.type_ {
        DictType::Str => a == b,
        _ => {
            // Compare key.size bytes
            let n = dict.key.size;
            if a.len() < n || b.len() < n {
                return a == b;
            }
            a[0..n] == b[0..n]
        }
    }
}

/// Pad a key byte slice to the dict's key.size, used for non-STR types.
fn padded_key(dict: &Dict, key_data: &[u8]) -> Vec<u8> {
    match dict.key.type_ {
        DictType::Str => key_data.to_vec(),
        _ => {
            let n = dict.key.size;
            let mut k = vec![0u8; n];
            let len = key_data.len().min(n);
            k[..len].copy_from_slice(&key_data[..len]);
            k
        }
    }
}

/// Create a dictionary using detailed arguments. Matches C's dict_create().
pub fn dict_create(args: DictArgs) -> Dict {
    let key_size = key_size_for(args.key.type_, args.key.size);
    let val_size = ptr_align(args.val.size);

    let mut key = args.key.clone();
    key.size = key_size;
    let mut val = args.val.clone();
    val.size = val_size;

    let mod_ = DEFAULT_MOD;
    let buckets = (0..mod_).map(|_| DictBucket { elements: Vec::new() }).collect();

    Dict {
        key,
        val,
        alloc: args.alloc,
        mod_,
        buckets,
        key_temp: vec![0u8; key_size],
        keys_dump: Vec::new(),
        count: 0,
    }
}

/// Create a dictionary with derived arguments. Matches C's dict_new().
pub fn dict_new(key_type: DictType, key_size: usize, val_size: usize) -> Dict {
    let args = DictArgs {
        key: DictKeyAttr {
            type_: key_type,
            size: key_size,
            copy: None,
            free: None,
            hash: None,
            cmpr: None,
        },
        val: DictValAttr {
            size: val_size,
            free: None,
        },
        alloc: DictAlloc { malloc: None, free: None },
    };
    dict_create(args)
}

/// Destroy a dictionary. Matches C's dict_destroy().
pub fn dict_destroy(dict: &mut Dict) {
    // Run destructors on keys/vals if provided
    let key_free = dict.key.free;
    let val_free = dict.val.free;
    let key_has_copy = dict.key.copy.is_some();
    for bucket in dict.buckets.iter_mut() {
        for elem in bucket.elements.iter_mut() {
            if key_has_copy {
                if let Some(kf) = key_free {
                    kf(&mut elem.key);
                }
            }
            if dict.val.size != 0 {
                if let Some(vf) = val_free {
                    vf(&mut elem.val);
                }
            }
        }
        bucket.elements.clear();
    }
    dict.buckets.clear();
    dict.count = 0;
}

/// Retrieve or create a value from the dictionary.
pub fn dict_get<'dict>(dict: &'dict mut Dict, key_data: &[u8]) -> Option<&'dict mut [u8]> {
    let key = padded_key(dict, key_data);
    let code = compute_hash(dict, &key);
    let index = (code % dict.mod_ as u64) as usize;

    // Search
    let mut found_pos: Option<usize> = None;
    {
        let bucket = &dict.buckets[index];
        for (i, elem) in bucket.elements.iter().enumerate() {
            if elem.code != code {
                continue;
            }
            if keys_equal(dict, &elem.key, &key) {
                found_pos = Some(i);
                break;
            }
        }
    }

    if let Some(pos) = found_pos {
        return Some(&mut dict.buckets[index].elements[pos].val);
    }

    // Insert new
    let val_size = dict.val.size;
    let new_elem = DictElem {
        code,
        key,
        val: vec![0u8; val_size],
    };
    dict.buckets[index].elements.push(new_elem);
    dict.count += 1;

    let bucket_size = dict.buckets[index].elements.len();
    let mod_ = dict.mod_;
    let need_reshape = bucket_size > mod_ + 1; // matches "size++ > mod" => post-increment compared to mod

    // Note: in C, `size++ > mod` is post-increment, so it triggers when previous size was > mod
    // i.e., when after insert size > mod + 1. But we already incremented. Let's match exactly:
    // C: `if ( dict->list[ index ].size++ > dict->mod )` -- post-increment, so condition is checked
    // with old value, then size is incremented. New size is old+1. So if old > mod, then new > mod+1.
    // Equivalent: bucket_size > mod_ + 1 (where bucket_size is new size).
    // But actually in C, size++ uses old value for compare then increments. So condition checks
    // old_size > mod. Old size = bucket_size - 1 (since we already pushed). So condition is
    // bucket_size - 1 > mod, i.e., bucket_size > mod + 1.

    if need_reshape {
        if !dict_reshape(dict, 1) {
            return None;
        }
        // After reshape, find the element again
        let new_index = (code % dict.mod_ as u64) as usize;
        let bucket = &mut dict.buckets[new_index];
        for elem in bucket.elements.iter_mut() {
            if elem.code == code {
                let key_for_compare = padded_key_static(dict.key.type_, dict.key.size, key_data);
                if keys_equal_for(&dict.key, &elem.key, &key_for_compare) {
                    return Some(&mut elem.val);
                }
            }
        }
        return None;
    }

    Some(&mut dict.buckets[index].elements[bucket_size - 1].val)
}

fn padded_key_static(type_: DictType, size: usize, key_data: &[u8]) -> Vec<u8> {
    match type_ {
        DictType::Str => key_data.to_vec(),
        _ => {
            let n = size;
            let mut k = vec![0u8; n];
            let len = key_data.len().min(n);
            k[..len].copy_from_slice(&key_data[..len]);
            k
        }
    }
}

fn keys_equal_for(key_attr: &DictKeyAttr, a: &[u8], b: &[u8]) -> bool {
    if let Some(cmpr) = key_attr.cmpr {
        return cmpr(a, b) == 0;
    }
    match key_attr.type_ {
        DictType::Str => a == b,
        _ => {
            let n = key_attr.size;
            if a.len() < n || b.len() < n {
                return a == b;
            }
            a[0..n] == b[0..n]
        }
    }
}

/// Remove a value from the dictionary.
pub fn dict_remove(dict: &mut Dict, key_data: &[u8]) -> bool {
    let key = padded_key(dict, key_data);
    let code = compute_hash(dict, &key);
    let index = (code % dict.mod_ as u64) as usize;

    let mut found_pos: Option<usize> = None;
    {
        let bucket = &dict.buckets[index];
        for (i, elem) in bucket.elements.iter().enumerate() {
            if elem.code != code {
                continue;
            }
            if keys_equal(dict, &elem.key, &key) {
                found_pos = Some(i);
                break;
            }
        }
    }

    if let Some(pos) = found_pos {
        let key_free = dict.key.free;
        let val_free = dict.val.free;
        let key_has_copy = dict.key.copy.is_some();
        let elem = &mut dict.buckets[index].elements[pos];
        if key_has_copy {
            if let Some(kf) = key_free {
                kf(&mut elem.key);
            }
        }
        if dict.val.size != 0 {
            if let Some(vf) = val_free {
                vf(&mut elem.val);
            }
        }
        dict.buckets[index].elements.remove(pos);
        dict.count -= 1;
        return true;
    }

    false
}

/// Check if a key exists in the dictionary.
pub fn dict_has(dict: &Dict, key_data: &[u8]) -> bool {
    let key = padded_key(dict, key_data);
    let code = compute_hash(dict, &key);
    let index = (code % dict.mod_ as u64) as usize;

    let bucket = &dict.buckets[index];
    for elem in bucket.elements.iter() {
        if elem.code != code {
            continue;
        }
        if keys_equal(dict, &elem.key, &key) {
            return true;
        }
    }
    false
}

/// Return the number of elements in the dictionary.
pub fn dict_len(dict: &Dict) -> usize {
    dict.count
}

/// Return a snapshot of all keys.
pub fn dict_key(dict: &Dict, size: &mut usize) -> Option<&'static [u8]> {
    *size = dict_len(dict);
    if *size == 0 {
        return None;
    }
    let mut keys: Vec<u8> = Vec::new();
    match dict.key.type_ {
        DictType::Str => {
            // Concatenate raw string bytes with no separator since each can have variable len
            for bucket in dict.buckets.iter() {
                for elem in bucket.elements.iter() {
                    keys.extend_from_slice(&elem.key);
                }
            }
        }
        _ => {
            let n = dict.key.size;
            for bucket in dict.buckets.iter() {
                for elem in bucket.elements.iter() {
                    let mut bytes = vec![0u8; n];
                    let len = elem.key.len().min(n);
                    bytes[..len].copy_from_slice(&elem.key[..len]);
                    keys.extend_from_slice(&bytes);
                }
            }
        }
    }
    let leaked: &'static [u8] = Box::leak(keys.into_boxed_slice());
    Some(leaked)
}

/// Serialize a dictionary into a contiguous Vec<u8>. Matches C's dict_serialize().
pub fn dict_serialize(dict: &Dict, bytes: &mut usize) -> Option<Vec<u8>> {
    let count = dict_len(dict) as u32;
    let key_size = dict.key.size as u32;
    let val_size = dict.val.size as u32;

    let elem_size: usize = if dict.key.type_ == DictType::Str {
        std::mem::size_of::<u32>() + dict.val.size
    } else {
        dict.key.size + dict.val.size
    };

    // Compute total size
    let mut total: usize = std::mem::size_of::<u32>() * 3 + (count as usize) * elem_size;

    // Collect string lengths if needed
    let mut strlen_table: Vec<u32> = Vec::new();
    if dict.key.type_ == DictType::Str {
        for bucket in dict.buckets.iter() {
            for elem in bucket.elements.iter() {
                let len = elem.key.len() as u32;
                strlen_table.push(len);
                total += len as usize;
            }
        }
    }

    *bytes = total;
    let mut data = vec![0u8; total];

    // Header
    let mut ptr = 0usize;
    data[ptr..ptr + 4].copy_from_slice(&key_size.to_ne_bytes());
    ptr += 4;
    data[ptr..ptr + 4].copy_from_slice(&val_size.to_ne_bytes());
    ptr += 4;
    data[ptr..ptr + 4].copy_from_slice(&count.to_ne_bytes());
    ptr += 4;

    if dict.key.type_ == DictType::Str {
        let mut str_ptr = ptr + (count as usize) * elem_size;
        let mut idx = 0usize;
        for bucket in dict.buckets.iter() {
            for elem in bucket.elements.iter() {
                let strlen = strlen_table[idx];
                data[ptr..ptr + 4].copy_from_slice(&strlen.to_ne_bytes());
                ptr += 4;
                // val
                let vlen = dict.val.size;
                data[ptr..ptr + vlen].copy_from_slice(&elem.val[..vlen]);
                ptr += vlen;
                // string body
                data[str_ptr..str_ptr + strlen as usize].copy_from_slice(&elem.key);
                str_ptr += strlen as usize;
                idx += 1;
            }
        }
    } else {
        let kn = dict.key.size;
        let vn = dict.val.size;
        for bucket in dict.buckets.iter() {
            for elem in bucket.elements.iter() {
                data[ptr..ptr + kn].copy_from_slice(&elem.key[..kn]);
                ptr += kn;
                data[ptr..ptr + vn].copy_from_slice(&elem.val[..vn]);
                ptr += vn;
            }
        }
    }

    Some(data)
}

/// Deserialize a dictionary from a slice. Matches C's dict_deserialize().
pub fn dict_deserialize(args: DictArgs, data: &[u8]) -> Dict {
    let mut header = [0u32; 3];
    for (i, h) in header.iter_mut().enumerate() {
        let off = i * 4;
        *h = u32::from_ne_bytes(data[off..off + 4].try_into().unwrap());
    }
    let mut ptr = 12usize;

    let key_size_expected = key_size_for(args.key.type_, args.key.size);
    let val_size_expected = ptr_align(args.val.size);

    if header[0] as usize != key_size_expected {
        // Return an empty dict in error case
        eprintln!("[ERRO]: key type conflict, data corrupted.");
        return dict_create(args);
    }
    if header[1] as usize != val_size_expected {
        eprintln!("[ERRO]: val type conflict, data corrupted.");
        return dict_create(args);
    }

    let mut dict = dict_create(args);
    let count = header[2] as usize;

    let elem_size: usize = if dict.key.type_ == DictType::Str {
        std::mem::size_of::<u32>() + dict.val.size
    } else {
        dict.key.size + dict.val.size
    };

    if dict.key.type_ == DictType::Str {
        let mut str_ptr = ptr + count * elem_size;
        for _ in 0..count {
            let strlen = u32::from_ne_bytes(data[ptr..ptr + 4].try_into().unwrap()) as usize;
            ptr += 4;
            let val_bytes = data[ptr..ptr + dict.val.size].to_vec();
            ptr += dict.val.size;
            let key_bytes = data[str_ptr..str_ptr + strlen].to_vec();
            str_ptr += strlen;
            // Insert directly
            let code = compute_hash(&dict, &key_bytes);
            let index = (code % dict.mod_ as u64) as usize;
            let elem = DictElem {
                code,
                key: key_bytes,
                val: val_bytes,
            };
            dict.buckets[index].elements.push(elem);
            dict.count += 1;
        }
    } else {
        let kn = dict.key.size;
        let vn = dict.val.size;
        for _ in 0..count {
            let key_bytes = data[ptr..ptr + kn].to_vec();
            ptr += kn;
            let val_bytes = data[ptr..ptr + vn].to_vec();
            ptr += vn;
            let code = compute_hash(&dict, &key_bytes);
            let index = (code % dict.mod_ as u64) as usize;
            let elem = DictElem {
                code,
                key: key_bytes,
                val: val_bytes,
            };
            dict.buckets[index].elements.push(elem);
            dict.count += 1;
        }
    }

    // Rebalance if any bucket too large
    let mut max = 0usize;
    for bucket in dict.buckets.iter() {
        if bucket.elements.len() > max {
            max = bucket.elements.len();
        }
    }
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

/// Compare two keys by hash & equality.
pub fn dict_key_equals(dict: &Dict, a: &[u8], b: &[u8]) -> bool {
    keys_equal(dict, a, b)
}

/// The original dict_free_val. Kept for signature consistency.
pub fn dict_free_val(dict: &Dict, val: &mut [u8]) {
    if let Some(vf) = dict.val.free {
        vf(val);
    }
}

/// Not used in pure Rust version, matching signature only.
pub fn dict_get_key(_dict: &Dict) -> Option<&mut [u8]> {
    None
}

/// Internal function to reshape the dictionary. Matches C's dict_reshape().
pub fn dict_reshape(dict: &mut Dict, step: usize) -> bool {
    let old_size = dict.mod_;
    let new_size = old_size * step * DEFAULT_STEP;
    if new_size == 0 {
        return false;
    }

    let mut new_buckets: Vec<DictBucket> = (0..new_size)
        .map(|_| DictBucket { elements: Vec::new() })
        .collect();

    let old_buckets = std::mem::replace(&mut dict.buckets, Vec::new());
    for mut bucket in old_buckets {
        for elem in bucket.elements.drain(..) {
            let index = (elem.code % new_size as u64) as usize;
            new_buckets[index].elements.push(elem);
        }
    }

    dict.mod_ = new_size;
    dict.buckets = new_buckets;
    true
}

/// Internal function to free a node.
pub fn dict_free_node(_dict: &Dict, _node: &mut DictElem) {
    // no-op: Rust drops allocations automatically
}

/// Internal function to free a dictionary key.
pub fn dict_free_key(dict: &Dict, key: &mut [u8]) {
    if dict.key.copy.is_some() {
        if let Some(kf) = dict.key.free {
            kf(key);
        }
    }
}

/// The original dict_get_hash.
pub fn dict_get_hash(dict: &Dict, key: &[u8]) -> u64 {
    compute_hash(dict, key)
}

/// Not used in this design, but signature is kept.
pub fn dict_delete_node(_list: &mut DictBucket, _curr: &mut DictElem) {
    // no-op in this safe design
}
