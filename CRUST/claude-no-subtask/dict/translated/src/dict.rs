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

/// Compute the platform-aligned key size based on the type and the requested size.
fn compute_key_size(attr: &DictKeyAttr) -> usize {
    match attr.type_ {
        DictType::Char => 1,
        DictType::WChar => 4, // wchar_t is 4 bytes on Linux
        DictType::I32 => 4,
        DictType::U32 => 4,
        DictType::F32 => 4,
        DictType::I64 => 8,
        DictType::U64 => 8,
        DictType::F64 => 8,
        DictType::Ptr => 8,
        DictType::Str => 8, // sizeof(char*) on 64-bit
        DictType::Struct => {
            let align = std::mem::size_of::<usize>();
            (attr.size + (align - 1)) & !(align - 1)
        }
    }
}

/// Compute the hash code for a given key. Uses dict.key.hash if provided, else
/// falls back to default hashing based on the key's type.
fn compute_hash(dict: &Dict, key_bytes: &[u8]) -> u64 {
    if let Some(hash_fn) = dict.key.hash {
        return hash_fn(key_bytes);
    }
    match dict.key.type_ {
        DictType::Char => {
            if key_bytes.is_empty() {
                0
            } else {
                // signed char on x86 promoted to int promoted to uint64_t
                (key_bytes[0] as i8) as i64 as u64
            }
        }
        DictType::WChar => {
            if key_bytes.len() < 4 {
                0
            } else {
                let arr: [u8; 4] = key_bytes[0..4].try_into().unwrap();
                i32::from_ne_bytes(arr) as u64
            }
        }
        DictType::I32 => {
            if key_bytes.len() < 4 {
                0
            } else {
                let arr: [u8; 4] = key_bytes[0..4].try_into().unwrap();
                i32::from_ne_bytes(arr) as u64
            }
        }
        DictType::U32 => {
            if key_bytes.len() < 4 {
                0
            } else {
                let arr: [u8; 4] = key_bytes[0..4].try_into().unwrap();
                u32::from_ne_bytes(arr) as u64
            }
        }
        DictType::F32 => {
            if key_bytes.len() < 4 {
                0
            } else {
                let arr: [u8; 4] = key_bytes[0..4].try_into().unwrap();
                let v = f32::from_ne_bytes(arr);
                // C: code = *(float*) key  -> truncates float to integer
                if v.is_nan() {
                    0
                } else {
                    v as u64
                }
            }
        }
        DictType::I64 => {
            if key_bytes.len() < 8 {
                0
            } else {
                let arr: [u8; 8] = key_bytes[0..8].try_into().unwrap();
                i64::from_ne_bytes(arr) as u64
            }
        }
        DictType::U64 => {
            if key_bytes.len() < 8 {
                0
            } else {
                let arr: [u8; 8] = key_bytes[0..8].try_into().unwrap();
                u64::from_ne_bytes(arr)
            }
        }
        DictType::F64 => {
            if key_bytes.len() < 8 {
                0
            } else {
                let arr: [u8; 8] = key_bytes[0..8].try_into().unwrap();
                let v = f64::from_ne_bytes(arr);
                if v.is_nan() {
                    0
                } else {
                    v as u64
                }
            }
        }
        DictType::Ptr => {
            if key_bytes.len() < 8 {
                0
            } else {
                let arr: [u8; 8] = key_bytes[0..8].try_into().unwrap();
                u64::from_ne_bytes(arr)
            }
        }
        DictType::Str => {
            // Match the C string hash: code = (code * 256 + key[i]) % HASH_MOD
            let mut code: u64 = 0;
            for &b in key_bytes.iter() {
                code = (code.wrapping_mul(HASH_BASE).wrapping_add(b as u64)) % HASH_MOD;
            }
            code
        }
        DictType::Struct => {
            let length = dict.key.size;
            let mut code: u64 = 0;
            for i in 0..length {
                let b = if i < key_bytes.len() { key_bytes[i] } else { 0 };
                code = (code.wrapping_mul(HASH_BASE).wrapping_add(b as u64)) % HASH_MOD;
            }
            code
        }
    }
}

/// Internal helper for comparing two keys.
fn keys_equal(dict: &Dict, a: &[u8], b: &[u8]) -> bool {
    if let Some(cmp_fn) = dict.key.cmpr {
        return cmp_fn(a, b) == 0;
    }
    a == b
}

/// Create a dictionary using detailed arguments. Matches C's dict_create().
pub fn dict_create(args: DictArgs) -> Dict {
    let key_size = compute_key_size(&args.key);
    let align = std::mem::size_of::<usize>();
    let val_size = (args.val.size + (align - 1)) & !(align - 1);

    let mut key_attr = args.key.clone();
    key_attr.size = key_size;
    let mut val_attr = args.val.clone();
    val_attr.size = val_size;

    Dict {
        key: key_attr,
        val: val_attr,
        alloc: args.alloc,
        mod_: DEFAULT_MOD,
        buckets: vec![DictBucket { elements: Vec::new() }; DEFAULT_MOD],
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
        val: DictValAttr {
            size: val_size,
            free: None,
        },
        alloc: DictAlloc {
            malloc: None,
            free: None,
        },
    })
}
/// Destroy a dictionary. Matches C's dict_destroy().
/// In Rust, memory is freed automatically, but we emulate calling destructors if provided.
pub fn dict_destroy(dict: &mut Dict) {
    let key_free = dict.key.free;
    let val_free = dict.val.free;
    let val_size = dict.val.size;
    for bucket in dict.buckets.iter_mut() {
        for elem in bucket.elements.iter_mut() {
            if let Some(f) = key_free {
                f(&mut elem.key);
            }
            if val_size != 0 {
                if let Some(f) = val_free {
                    f(&mut elem.val);
                }
            }
        }
        bucket.elements.clear();
    }
    dict.buckets.clear();
    dict.count = 0;
}
/// Retrieve or create a value from the dictionary. In C, this used varargs. Here, we
/// accept a slice of bytes as the key. Returns a mutable slice of the value,
/// or None if something went wrong. Matches C's dict_get(dict_t*, ...).
pub fn dict_get<'dict>(dict: &'dict mut Dict, key_data: &[u8]) -> Option<&'dict mut [u8]> {
    let code = compute_hash(dict, key_data);
    let index = (code as usize) % dict.mod_;

    // Search for an existing element
    let mut found_idx: Option<usize> = None;
    let bucket_len = dict.buckets[index].elements.len();
    for i in 0..bucket_len {
        let matches = {
            let elem = &dict.buckets[index].elements[i];
            elem.code == code && keys_equal(dict, &elem.key, key_data)
        };
        if matches {
            found_idx = Some(i);
            break;
        }
    }

    if found_idx.is_none() {
        // Insert new element
        let new_elem = DictElem {
            code,
            key: key_data.to_vec(),
            val: vec![0u8; dict.val.size],
        };
        dict.buckets[index].elements.push(new_elem);
        dict.count += 1;

        let new_size = dict.buckets[index].elements.len();
        if new_size > dict.mod_ {
            if !dict_reshape(dict, 1) {
                return None;
            }
        }
    }

    // Final search for the element (after possible reshape)
    let new_index = (code as usize) % dict.mod_;
    let bucket_len = dict.buckets[new_index].elements.len();
    let mut final_idx: Option<usize> = None;
    for i in 0..bucket_len {
        let matches = {
            let elem = &dict.buckets[new_index].elements[i];
            elem.code == code && keys_equal(dict, &elem.key, key_data)
        };
        if matches {
            final_idx = Some(i);
            break;
        }
    }
    let i = final_idx?;
    Some(&mut dict.buckets[new_index].elements[i].val[..])
}
/// Remove a value from the dictionary. Matches C's dict_remove(dict_t*, ...).
/// Returns true if the element was found and removed.
pub fn dict_remove(dict: &mut Dict, key_data: &[u8]) -> bool {
    let code = compute_hash(dict, key_data);
    let index = (code as usize) % dict.mod_;

    let mut found_idx: Option<usize> = None;
    let bucket_len = dict.buckets[index].elements.len();
    for i in 0..bucket_len {
        let matches = {
            let elem = &dict.buckets[index].elements[i];
            elem.code == code && keys_equal(dict, &elem.key, key_data)
        };
        if matches {
            found_idx = Some(i);
            break;
        }
    }

    let key_free = dict.key.free;
    let val_free = dict.val.free;
    let val_size = dict.val.size;

    if let Some(i) = found_idx {
        let mut elem = dict.buckets[index].elements.remove(i);
        if let Some(f) = key_free {
            f(&mut elem.key);
        }
        if val_size != 0 {
            if let Some(f) = val_free {
                f(&mut elem.val);
            }
        }
        dict.count -= 1;
        true
    } else {
        false
    }
}
/// Check if a key exists in the dictionary. Matches C's dict_has(const dict_t*, ...).
pub fn dict_has(dict: &Dict, key_data: &[u8]) -> bool {
    let code = compute_hash(dict, key_data);
    let index = (code as usize) % dict.mod_;

    for elem in dict.buckets[index].elements.iter() {
        if elem.code != code {
            continue;
        }
        if keys_equal(dict, &elem.key, key_data) {
            return true;
        }
    }
    false
}
/// Return the number of elements in the dictionary. Matches C's dict_len().
pub fn dict_len(dict: &Dict) -> usize {
    let mut total = 0;
    for bucket in dict.buckets.iter() {
        total += bucket.elements.len();
    }
    total
}
/// Return a snapshot of all keys.
pub fn dict_key(dict: &Dict, size: &mut usize) -> Option<&'static [u8]> {
    let count = dict_len(dict);
    *size = count;
    if count == 0 {
        return None;
    }

    let mut buffer: Vec<u8> = Vec::new();
    for bucket in dict.buckets.iter() {
        for elem in bucket.elements.iter() {
            buffer.extend_from_slice(&elem.key);
        }
    }

    let leaked: &'static [u8] = Box::leak(buffer.into_boxed_slice());
    Some(leaked)
}
/// Serialize a dictionary into a contiguous Vec<u8>. Matches C's dict_serialize().
pub fn dict_serialize(dict: &Dict, bytes: &mut usize) -> Option<Vec<u8>> {
    let size = dict_len(dict) as u32;
    let key_size = dict.key.size as u32;
    let val_size = dict.val.size as u32;

    let elem_size = if dict.key.type_ == DictType::Str {
        4 + dict.val.size
    } else {
        dict.key.size + dict.val.size
    };

    let mut total_size = 12 + (size as usize) * elem_size;

    // Compute string lengths for STR type and add to total_size
    let mut strlen_table: Vec<u32> = Vec::new();
    if dict.key.type_ == DictType::Str {
        for bucket in dict.buckets.iter() {
            for elem in bucket.elements.iter() {
                let len = elem.key.len() as u32;
                strlen_table.push(len);
                total_size += len as usize;
            }
        }
    }

    *bytes = total_size;

    let mut data: Vec<u8> = Vec::with_capacity(total_size);
    data.extend_from_slice(&key_size.to_ne_bytes());
    data.extend_from_slice(&val_size.to_ne_bytes());
    data.extend_from_slice(&size.to_ne_bytes());

    if dict.key.type_ == DictType::Str {
        let mut elem_data: Vec<u8> = Vec::new();
        let mut str_data: Vec<u8> = Vec::new();
        let mut idx = 0;
        for bucket in dict.buckets.iter() {
            for elem in bucket.elements.iter() {
                let len = strlen_table[idx];
                elem_data.extend_from_slice(&len.to_ne_bytes());
                elem_data.extend_from_slice(&elem.val);
                str_data.extend_from_slice(&elem.key);
                idx += 1;
            }
        }
        data.extend_from_slice(&elem_data);
        data.extend_from_slice(&str_data);
    } else {
        for bucket in dict.buckets.iter() {
            for elem in bucket.elements.iter() {
                // Ensure each key is exactly key.size bytes
                if elem.key.len() == dict.key.size {
                    data.extend_from_slice(&elem.key);
                } else {
                    let mut padded = vec![0u8; dict.key.size];
                    let copy_len = elem.key.len().min(dict.key.size);
                    padded[..copy_len].copy_from_slice(&elem.key[..copy_len]);
                    data.extend_from_slice(&padded);
                }
                data.extend_from_slice(&elem.val);
            }
        }
    }

    Some(data)
}
/// Deserialize a dictionary from a slice. Matches C's dict_deserialize().
pub fn dict_deserialize(args: DictArgs, data: &[u8]) -> Dict {
    if data.len() < 12 {
        return dict_create(args);
    }
    let key_size_in =
        u32::from_ne_bytes(data[0..4].try_into().unwrap()) as usize;
    let val_size_in =
        u32::from_ne_bytes(data[4..8].try_into().unwrap()) as usize;
    let count =
        u32::from_ne_bytes(data[8..12].try_into().unwrap()) as usize;

    let expected_key_size = compute_key_size(&args.key);
    let align = std::mem::size_of::<usize>();
    let expected_val_size = (args.val.size + (align - 1)) & !(align - 1);

    if expected_key_size != key_size_in {
        eprintln!("[ERRO]: key type conflict, data corrupted.");
        return dict_create(args);
    }
    if expected_val_size != val_size_in {
        eprintln!("[ERRO]: val type conflict, data corrupted.");
        return dict_create(args);
    }

    let mut dict = dict_create(args);

    let elem_size = if dict.key.type_ == DictType::Str {
        4 + dict.val.size
    } else {
        dict.key.size + dict.val.size
    };

    let mut ptr: usize = 12;

    if dict.key.type_ == DictType::Str {
        let mut str_ptr = 12 + count * elem_size;
        for _ in 0..count {
            if ptr + 4 > data.len() {
                break;
            }
            let strlen =
                u32::from_ne_bytes(data[ptr..ptr + 4].try_into().unwrap()) as usize;
            ptr += 4;
            if ptr + dict.val.size > data.len() {
                break;
            }
            let val_bytes = data[ptr..ptr + dict.val.size].to_vec();
            ptr += dict.val.size;

            if str_ptr + strlen > data.len() {
                break;
            }
            let key_bytes = data[str_ptr..str_ptr + strlen].to_vec();
            str_ptr += strlen;

            let code = compute_hash(&dict, &key_bytes);
            let index = (code as usize) % dict.mod_;
            dict.buckets[index].elements.push(DictElem {
                code,
                key: key_bytes,
                val: val_bytes,
            });
            dict.count += 1;
        }
    } else {
        for _ in 0..count {
            if ptr + dict.key.size > data.len() {
                break;
            }
            let key_bytes = data[ptr..ptr + dict.key.size].to_vec();
            ptr += dict.key.size;
            if ptr + dict.val.size > data.len() {
                break;
            }
            let val_bytes = data[ptr..ptr + dict.val.size].to_vec();
            ptr += dict.val.size;

            let code = compute_hash(&dict, &key_bytes);
            let index = (code as usize) % dict.mod_;
            dict.buckets[index].elements.push(DictElem {
                code,
                key: key_bytes,
                val: val_bytes,
            });
            dict.count += 1;
        }
    }

    // Reshape if any bucket grew too large
    let mut max = 0;
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
/// Convenience function to create a dictionary using inline arguments, mirroring the
/// C macro dict_create_args(...).
pub fn dict_create_args(args: DictArgs) -> Dict {
    dict_create(args)
}
/// The original dict_key_equals. Kept for signature consistency but not used internally
/// to avoid borrow conflicts.
pub fn dict_key_equals(dict: &Dict, a: &[u8], b: &[u8]) -> bool {
    keys_equal(dict, a, b)
}
/// Not used in this design, but signature is kept.
pub fn dict_delete_node(_list: &mut DictBucket, _curr: &mut DictElem) {
// no-op in this safe design
}
/// The original dict_free_val. Kept for signature consistency.
pub fn dict_free_val(dict: &Dict, val: &mut [u8]) {
    if let Some(f) = dict.val.free {
        f(val);
    }
}
/// Not used in pure Rust version, matching signature only.
pub fn dict_get_key(_dict: &Dict) -> Option<&mut [u8]> {
    None
}
/// Internal function to reshape the dictionary. Matches C's dict_reshape().
/// We re-allocate and re-hash all elements with new capacity = old * step * DEFAULT_STEP.
pub fn dict_reshape(dict: &mut Dict, step: usize) -> bool {
    let old_size = dict.mod_;
    let new_size = old_size * step * DEFAULT_STEP;
    if new_size == 0 {
        return false;
    }

    let mut new_buckets: Vec<DictBucket> =
        vec![DictBucket { elements: Vec::new() }; new_size];

    let old_buckets = std::mem::replace(&mut dict.buckets, Vec::new());
    for bucket in old_buckets {
        for elem in bucket.elements {
            let new_index = (elem.code as usize) % new_size;
            new_buckets[new_index].elements.push(elem);
        }
    }

    dict.buckets = new_buckets;
    dict.mod_ = new_size;
    true
}
/// Internal function to free a node. Matches C's dict_free_node().
pub fn dict_free_node(_dict: &Dict, _node: &mut DictElem) {
    // Memory is automatically freed when the DictElem is dropped.
}
/// Internal function to free a dictionary key. Kept for signature consistency.
pub fn dict_free_key(dict: &Dict, key: &mut [u8]) {
    if let Some(f) = dict.key.free {
        f(key);
    }
}
/// The original dict_get_hash. Kept for signature consistency but not used internally
/// to avoid borrow conflicts.
pub fn dict_get_hash(dict: &Dict, key: &[u8]) -> u64 {
    compute_hash(dict, key)
}
