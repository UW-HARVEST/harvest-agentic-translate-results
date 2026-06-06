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

// =============================================================================
// Internal helpers
// =============================================================================

fn align_to_uintptr(size: usize) -> usize {
    let align = std::mem::size_of::<usize>();
    if size == 0 {
        0
    } else {
        (size + align - 1) & !(align - 1)
    }
}

fn key_natural_size(t: DictType, struct_size: usize) -> usize {
    match t {
        DictType::Char => std::mem::size_of::<u8>(),
        DictType::WChar => 4, // wchar_t is 4 bytes on Linux
        DictType::I32 => std::mem::size_of::<i32>(),
        DictType::U32 => std::mem::size_of::<u32>(),
        DictType::F32 => std::mem::size_of::<f32>(),
        DictType::I64 => std::mem::size_of::<i64>(),
        DictType::U64 => std::mem::size_of::<u64>(),
        DictType::F64 => std::mem::size_of::<f64>(),
        DictType::Ptr => std::mem::size_of::<usize>(),
        DictType::Str => std::mem::size_of::<usize>(),
        DictType::Struct => align_to_uintptr(struct_size),
    }
}

fn compute_hash_internal(
    key_type: DictType,
    user_hash: Option<DictHash>,
    key: &[u8],
) -> u64 {
    if let Some(hash_fn) = user_hash {
        return hash_fn(key);
    }
    match key_type {
        DictType::Char => {
            // C char is signed on x86-64 Linux, sign-extend to u64
            if key.is_empty() {
                0
            } else {
                (key[0] as i8) as i64 as u64
            }
        }
        DictType::WChar => {
            let mut bytes = [0u8; 4];
            let n = key.len().min(4);
            bytes[..n].copy_from_slice(&key[..n]);
            // wchar_t treated like int32_t for hashing (sign-extended)
            i32::from_ne_bytes(bytes) as i64 as u64
        }
        DictType::I32 => {
            let mut bytes = [0u8; 4];
            let n = key.len().min(4);
            bytes[..n].copy_from_slice(&key[..n]);
            i32::from_ne_bytes(bytes) as i64 as u64
        }
        DictType::U32 => {
            let mut bytes = [0u8; 4];
            let n = key.len().min(4);
            bytes[..n].copy_from_slice(&key[..n]);
            u32::from_ne_bytes(bytes) as u64
        }
        DictType::F32 => {
            let mut bytes = [0u8; 4];
            let n = key.len().min(4);
            bytes[..n].copy_from_slice(&key[..n]);
            let f = f32::from_ne_bytes(bytes);
            f as u64
        }
        DictType::I64 => {
            let mut bytes = [0u8; 8];
            let n = key.len().min(8);
            bytes[..n].copy_from_slice(&key[..n]);
            i64::from_ne_bytes(bytes) as u64
        }
        DictType::U64 => {
            let mut bytes = [0u8; 8];
            let n = key.len().min(8);
            bytes[..n].copy_from_slice(&key[..n]);
            u64::from_ne_bytes(bytes)
        }
        DictType::F64 => {
            let mut bytes = [0u8; 8];
            let n = key.len().min(8);
            bytes[..n].copy_from_slice(&key[..n]);
            let f = f64::from_ne_bytes(bytes);
            f as u64
        }
        DictType::Ptr => {
            let mut bytes = [0u8; 8];
            let n = key.len().min(8);
            bytes[..n].copy_from_slice(&key[..n]);
            u64::from_ne_bytes(bytes)
        }
        DictType::Str | DictType::Struct => {
            let mut code: u64 = 0;
            for &b in key.iter() {
                code = (code.wrapping_mul(HASH_BASE).wrapping_add(b as u64)) % HASH_MOD;
            }
            code
        }
    }
}

fn keys_equal_internal(
    key_type: DictType,
    cmpr: Option<DictCmpr>,
    a: &[u8],
    b: &[u8],
) -> bool {
    if let Some(cmp) = cmpr {
        return cmp(a, b) == 0;
    }
    match key_type {
        DictType::Str => a == b,
        _ => a == b,
    }
}

fn build_key_bytes(dict: &Dict, key_data: &[u8]) -> Vec<u8> {
    if let Some(copy_fn) = dict.key.copy {
        let mut buf = vec![0u8; dict.key.size];
        copy_fn(&mut buf, key_data);
        buf
    } else {
        match dict.key.type_ {
            DictType::Str => key_data.to_vec(),
            _ => {
                let mut buf = vec![0u8; dict.key.size];
                let n = key_data.len().min(dict.key.size);
                buf[..n].copy_from_slice(&key_data[..n]);
                buf
            }
        }
    }
}

// =============================================================================
// Public API
// =============================================================================

/// Create a dictionary using detailed arguments. Matches C's dict_create().
pub fn dict_create(args: DictArgs) -> Dict {
    let key_size = key_natural_size(args.key.type_, args.key.size);
    let val_size = align_to_uintptr(args.val.size);

    let mut buckets: Vec<DictBucket> = Vec::with_capacity(DEFAULT_MOD);
    for _ in 0..DEFAULT_MOD {
        buckets.push(DictBucket { elements: Vec::new() });
    }

    let mut key_attr = args.key.clone();
    key_attr.size = key_size;

    let mut val_attr = args.val.clone();
    val_attr.size = val_size;

    Dict {
        key: key_attr,
        val: val_attr,
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
    let key_copy = dict.key.copy;
    let val_free = dict.val.free;
    let val_size = dict.val.size;

    for bucket in dict.buckets.iter_mut() {
        for elem in bucket.elements.iter_mut() {
            // Free the key if a custom destructor is provided
            if key_copy.is_some() {
                if let Some(f) = key_free {
                    f(&mut elem.key);
                }
            }
            // Free the value's inner allocation if destructor provided
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
    dict.mod_ = 0;
    dict.key_temp.clear();
}

/// Retrieve or create a value from the dictionary.
pub fn dict_get<'dict>(dict: &'dict mut Dict, key_data: &[u8]) -> Option<&'dict mut [u8]> {
    let key_type = dict.key.type_;
    let user_hash = dict.key.hash;
    let user_cmpr = dict.key.cmpr;

    // Build/normalize the candidate key bytes
    let candidate_key = build_key_bytes(dict, key_data);
    let code = compute_hash_internal(key_type, user_hash, &candidate_key);
    let index = (code % dict.mod_ as u64) as usize;

    // Look for an existing element
    let pos = dict.buckets[index]
        .elements
        .iter()
        .position(|elem| {
            elem.code == code && keys_equal_internal(key_type, user_cmpr, &elem.key, &candidate_key)
        });

    if let Some(p) = pos {
        // Found existing - candidate_key drops automatically.
        return Some(&mut dict.buckets[index].elements[p].val);
    }

    // Insert new element. Clone candidate_key so we can use it for re-lookup
    // after a potential reshape.
    let new_elem = DictElem {
        code,
        key: candidate_key.clone(),
        val: vec![0u8; dict.val.size],
    };
    dict.buckets[index].elements.push(new_elem);
    dict.count += 1;
    let bucket_size = dict.buckets[index].elements.len();
    let mod_ = dict.mod_;

    // Reshape if needed (mirrors C: trigger after pre-increment value > mod).
    // pre-increment was bucket_size - 1, so trigger when (bucket_size - 1) > mod.
    if bucket_size > 0 && (bucket_size - 1) > mod_ {
        if !dict_reshape(dict, 1) {
            return None;
        }
    }

    // Find element again (may have moved due to reshape).
    let new_index = (code % dict.mod_ as u64) as usize;
    let p = dict.buckets[new_index]
        .elements
        .iter()
        .position(|elem| {
            elem.code == code
                && keys_equal_internal(key_type, user_cmpr, &elem.key, &candidate_key)
        })?;
    Some(&mut dict.buckets[new_index].elements[p].val)
}

/// Remove a value from the dictionary.
pub fn dict_remove(dict: &mut Dict, key_data: &[u8]) -> bool {
    let key_type = dict.key.type_;
    let user_hash = dict.key.hash;
    let user_cmpr = dict.key.cmpr;
    let key_copy = dict.key.copy;
    let key_free = dict.key.free;
    let val_free = dict.val.free;
    let val_size = dict.val.size;

    let candidate_key = build_key_bytes(dict, key_data);
    let code = compute_hash_internal(key_type, user_hash, &candidate_key);
    let index = (code % dict.mod_ as u64) as usize;

    let pos = dict.buckets[index]
        .elements
        .iter()
        .position(|elem| {
            elem.code == code
                && keys_equal_internal(key_type, user_cmpr, &elem.key, &candidate_key)
        });

    if let Some(p) = pos {
        let mut removed = dict.buckets[index].elements.remove(p);
        // Free the elem's key if needed
        if key_copy.is_some() {
            if let Some(f) = key_free {
                f(&mut removed.key);
            }
        }
        // Free the value's inner allocation
        if val_size != 0 {
            if let Some(f) = val_free {
                f(&mut removed.val);
            }
        }
        dict.count -= 1;
        true
    } else {
        false
    }
}

/// Check if a key exists in the dictionary.
pub fn dict_has(dict: &Dict, key_data: &[u8]) -> bool {
    let key_type = dict.key.type_;
    let user_hash = dict.key.hash;
    let user_cmpr = dict.key.cmpr;

    let candidate_key = if let Some(copy_fn) = dict.key.copy {
        let mut buf = vec![0u8; dict.key.size];
        copy_fn(&mut buf, key_data);
        buf
    } else {
        match key_type {
            DictType::Str => key_data.to_vec(),
            _ => {
                let mut buf = vec![0u8; dict.key.size];
                let n = key_data.len().min(dict.key.size);
                buf[..n].copy_from_slice(&key_data[..n]);
                buf
            }
        }
    };
    let code = compute_hash_internal(key_type, user_hash, &candidate_key);
    let index = (code % dict.mod_ as u64) as usize;

    dict.buckets[index].elements.iter().any(|elem| {
        elem.code == code && keys_equal_internal(key_type, user_cmpr, &elem.key, &candidate_key)
    })
}

/// Return the number of elements in the dictionary.
pub fn dict_len(dict: &Dict) -> usize {
    dict.buckets.iter().map(|b| b.elements.len()).sum()
}

/// Return a snapshot of all keys in the dictionary as a leaked &'static [u8].
pub fn dict_key(dict: &Dict, size: &mut usize) -> Option<&'static [u8]> {
    let count = dict_len(dict);
    *size = count;
    if count == 0 {
        return None;
    }

    let mut buf: Vec<u8> = Vec::new();
    if dict.key.type_ == DictType::Str {
        // Concatenate strings null-terminated for caller convenience
        for bucket in dict.buckets.iter() {
            for elem in bucket.elements.iter() {
                buf.extend_from_slice(&elem.key);
                buf.push(0);
            }
        }
    } else {
        for bucket in dict.buckets.iter() {
            for elem in bucket.elements.iter() {
                // Copy exactly key.size bytes
                if elem.key.len() >= dict.key.size {
                    buf.extend_from_slice(&elem.key[..dict.key.size]);
                } else {
                    buf.extend_from_slice(&elem.key);
                    buf.resize(buf.len() + (dict.key.size - elem.key.len()), 0);
                }
            }
        }
    }

    let leaked: &'static mut [u8] = Box::leak(buf.into_boxed_slice());
    Some(leaked)
}

/// Serialize a dictionary into a contiguous Vec<u8>.
pub fn dict_serialize(dict: &Dict, bytes: &mut usize) -> Option<Vec<u8>> {
    let count = dict_len(dict) as u32;
    let key_size_u32 = dict.key.size as u32;
    let val_size_u32 = dict.val.size as u32;

    let elem_size = if dict.key.type_ == DictType::Str {
        std::mem::size_of::<u32>() + dict.val.size
    } else {
        dict.key.size + dict.val.size
    };

    // Compute total size
    let mut total = std::mem::size_of::<u32>() * 3 + (count as usize) * elem_size;

    // For STR: also include the string data bytes
    let mut strlen_table: Vec<u32> = Vec::new();
    if dict.key.type_ == DictType::Str {
        strlen_table.reserve(count as usize);
        for bucket in dict.buckets.iter() {
            for elem in bucket.elements.iter() {
                let len = elem.key.len() as u32;
                strlen_table.push(len);
                total += len as usize;
            }
        }
    }

    *bytes = total;

    let mut data: Vec<u8> = Vec::with_capacity(total);

    // Header
    data.extend_from_slice(&key_size_u32.to_ne_bytes());
    data.extend_from_slice(&val_size_u32.to_ne_bytes());
    data.extend_from_slice(&count.to_ne_bytes());

    if dict.key.type_ == DictType::Str {
        // First: write all (str_len, val) entries
        // Second: append all string bytes at the end
        let mut idx = 0usize;
        let mut str_data: Vec<u8> = Vec::new();
        for bucket in dict.buckets.iter() {
            for elem in bucket.elements.iter() {
                let str_len = strlen_table[idx];
                data.extend_from_slice(&str_len.to_ne_bytes());
                // val bytes (pad/truncate to val.size)
                if elem.val.len() >= dict.val.size {
                    data.extend_from_slice(&elem.val[..dict.val.size]);
                } else {
                    data.extend_from_slice(&elem.val);
                    data.resize(data.len() + (dict.val.size - elem.val.len()), 0);
                }
                str_data.extend_from_slice(&elem.key);
                idx += 1;
            }
        }
        data.extend_from_slice(&str_data);
    } else {
        for bucket in dict.buckets.iter() {
            for elem in bucket.elements.iter() {
                // key bytes
                if elem.key.len() >= dict.key.size {
                    data.extend_from_slice(&elem.key[..dict.key.size]);
                } else {
                    data.extend_from_slice(&elem.key);
                    data.resize(data.len() + (dict.key.size - elem.key.len()), 0);
                }
                // val bytes
                if elem.val.len() >= dict.val.size {
                    data.extend_from_slice(&elem.val[..dict.val.size]);
                } else {
                    data.extend_from_slice(&elem.val);
                    data.resize(data.len() + (dict.val.size - elem.val.len()), 0);
                }
            }
        }
    }

    Some(data)
}

/// Deserialize a dictionary from a slice.
pub fn dict_deserialize(args: DictArgs, data: &[u8]) -> Dict {
    let key_size_in = u32::from_ne_bytes(data[0..4].try_into().unwrap()) as usize;
    let val_size_in = u32::from_ne_bytes(data[4..8].try_into().unwrap()) as usize;
    let count = u32::from_ne_bytes(data[8..12].try_into().unwrap()) as usize;
    let mut ptr = 12usize;

    let expected_key_size = key_natural_size(args.key.type_, args.key.size);
    let expected_val_size = align_to_uintptr(args.val.size);

    if expected_key_size != key_size_in {
        panic!(
            "[ERRO]: key type conflict, data corrupted. expected={}, got={}",
            expected_key_size, key_size_in
        );
    }
    if expected_val_size != val_size_in {
        panic!(
            "[ERRO]: val type conflict, data corrupted. expected={}, got={}",
            expected_val_size, val_size_in
        );
    }

    let mut dict = dict_create(args);
    let key_type = dict.key.type_;
    let user_hash = dict.key.hash;
    let key_size = dict.key.size;
    let val_size = dict.val.size;

    if key_type == DictType::Str {
        let elem_size = std::mem::size_of::<u32>() + val_size;
        let str_section_start = ptr + count * elem_size;
        let mut str_offset = 0usize;

        // Stash entries first (so we can do hash-based insertion afterward)
        let mut entries: Vec<(Vec<u8>, Vec<u8>)> = Vec::with_capacity(count);
        for _ in 0..count {
            let str_len =
                u32::from_ne_bytes(data[ptr..ptr + 4].try_into().unwrap()) as usize;
            ptr += 4;
            let val_bytes = data[ptr..ptr + val_size].to_vec();
            ptr += val_size;
            let str_bytes = data[str_section_start + str_offset
                ..str_section_start + str_offset + str_len]
                .to_vec();
            str_offset += str_len;
            entries.push((str_bytes, val_bytes));
        }

        for (key_bytes, val_bytes) in entries {
            let code = compute_hash_internal(key_type, user_hash, &key_bytes);
            let index = (code % dict.mod_ as u64) as usize;
            dict.buckets[index].elements.push(DictElem {
                code,
                key: key_bytes,
                val: val_bytes,
            });
            dict.count += 1;
        }
    } else {
        for _ in 0..count {
            let key_bytes = data[ptr..ptr + key_size].to_vec();
            ptr += key_size;
            let val_bytes = data[ptr..ptr + val_size].to_vec();
            ptr += val_size;
            let code = compute_hash_internal(key_type, user_hash, &key_bytes);
            let index = (code % dict.mod_ as u64) as usize;
            dict.buckets[index].elements.push(DictElem {
                code,
                key: key_bytes,
                val: val_bytes,
            });
            dict.count += 1;
        }
    }

    // Reshape if any bucket grew too big
    let max = dict
        .buckets
        .iter()
        .map(|b| b.elements.len())
        .max()
        .unwrap_or(0);
    if max > DEFAULT_MOD {
        let step = max / DEFAULT_MOD;
        if step > 0 {
            dict_reshape(&mut dict, step);
        }
    }
    dict
}

/// Convenience function to create a dictionary using inline arguments, mirroring the
/// C macro dict_create_args(...).
pub fn dict_create_args(args: DictArgs) -> Dict {
    dict_create(args)
}

/// The original dict_key_equals.
pub fn dict_key_equals(dict: &Dict, a: &[u8], b: &[u8]) -> bool {
    keys_equal_internal(dict.key.type_, dict.key.cmpr, a, b)
}

/// The original dict_free_val.
pub fn dict_free_val(dict: &Dict, val: &mut [u8]) {
    if let Some(f) = dict.val.free {
        f(val);
    }
}

/// Not used in pure Rust version.
pub fn dict_get_key(_dict: &Dict) -> Option<&mut [u8]> {
    None
}

/// Internal function to reshape the dictionary.
pub fn dict_reshape(dict: &mut Dict, step: usize) -> bool {
    let old_size = dict.mod_;
    let new_size = old_size * step * DEFAULT_STEP;
    if new_size == 0 {
        return false;
    }

    let mut new_buckets: Vec<DictBucket> = Vec::with_capacity(new_size);
    for _ in 0..new_size {
        new_buckets.push(DictBucket { elements: Vec::new() });
    }

    let old_buckets = std::mem::replace(&mut dict.buckets, Vec::new());
    dict.mod_ = new_size;
    dict.buckets = new_buckets;

    for bucket in old_buckets {
        for elem in bucket.elements {
            let index = (elem.code % new_size as u64) as usize;
            dict.buckets[index].elements.push(elem);
        }
    }
    true
}

/// Internal function to free a node.
pub fn dict_free_node(_dict: &Dict, _node: &mut DictElem) {
    // No-op in safe Rust; Vec drops automatically.
}

/// Internal function to free a dictionary key.
pub fn dict_free_key(dict: &Dict, key: &mut [u8]) {
    if dict.key.copy.is_some() {
        if let Some(f) = dict.key.free {
            f(key);
        }
    }
}

/// The original dict_get_hash.
pub fn dict_get_hash(dict: &Dict, key: &[u8]) -> u64 {
    compute_hash_internal(dict.key.type_, dict.key.hash, key)
}

// Suppress unused-import warnings for the "boilerplate" imports kept to match
// the original interface header.
#[allow(dead_code)]
fn _unused_imports_anchor() {
    let _ = Ordering::Equal;
    let _: Option<DefaultHasher> = None;
    fn _hash_anchor<T: StdHash>(_t: &T) {
        let h = DefaultHasher::new();
        let _ = h.finish();
    }
    let _: Option<Mutex<u8>> = None;
}
