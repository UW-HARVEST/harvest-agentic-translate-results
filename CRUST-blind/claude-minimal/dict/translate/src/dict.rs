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

/// Internal helper: align a value up to the size of `usize` (matches the C
/// alignment used for struct/val sizes).
fn align_up(size: usize) -> usize {
    let align = std::mem::size_of::<usize>();
    (size + align - 1) & !(align - 1)
}

/// Internal helper: compute the storage size of the key based on its type.
fn key_storage_size(type_: DictType, size: usize) -> usize {
    match type_ {
        DictType::Char => 1,
        DictType::WChar => 4,
        DictType::I32 => 4,
        DictType::U32 => 4,
        DictType::F32 => 4,
        DictType::I64 => 8,
        DictType::U64 => 8,
        DictType::F64 => 8,
        DictType::Ptr => std::mem::size_of::<usize>(),
        DictType::Str => std::mem::size_of::<usize>(),
        DictType::Struct => align_up(size),
    }
}

/// Build the canonical "key bytes" used to store/lookup an entry. For types
/// with a fixed binary representation this is just the raw bytes, padded to
/// `dict.key.size`. For DICT_STR, we use the actual string bytes (no null
/// terminator). If a custom copy callback is supplied we route the bytes
/// through it.
fn make_key_bytes(dict: &Dict, key_data: &[u8]) -> Vec<u8> {
    if let Some(copy_fn) = dict.key.copy {
        let mut buf = vec![0u8; dict.key.size];
        copy_fn(&mut buf, key_data);
        return buf;
    }
    match dict.key.type_ {
        DictType::Str => key_data.to_vec(),
        _ => {
            let mut buf = vec![0u8; dict.key.size];
            let n = key_data.len().min(buf.len());
            buf[..n].copy_from_slice(&key_data[..n]);
            buf
        }
    }
}

/// Internal helper: compare two stored keys. Mirrors the comparison logic in
/// the C source (custom cmpr if provided, otherwise byte-wise equality).
fn keys_equal(dict: &Dict, a: &[u8], b: &[u8]) -> bool {
    if let Some(cmpr_fn) = dict.key.cmpr {
        return cmpr_fn(a, b) == 0;
    }
    a == b
}

/// Create a dictionary using detailed arguments. Matches C's dict_create().
pub fn dict_create(args: DictArgs) -> Dict {
    let DictArgs { key: key_attr, val: val_attr, alloc } = args;

    let key_size = key_storage_size(key_attr.type_, key_attr.size);
    let val_size = align_up(val_attr.size);

    let mut key = key_attr;
    key.size = key_size;

    let mut val = val_attr;
    val.size = val_size;

    let mod_ = DEFAULT_MOD;
    let mut buckets = Vec::with_capacity(mod_);
    for _ in 0..mod_ {
        buckets.push(DictBucket { elements: Vec::new() });
    }

    Dict {
        key,
        val,
        alloc,
        mod_,
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
    let key_copy = dict.key.copy;
    let key_free = dict.key.free;
    let val_free = dict.val.free;

    for bucket in dict.buckets.iter_mut() {
        for elem in bucket.elements.iter_mut() {
            if key_copy.is_some() {
                if let Some(kf) = key_free {
                    kf(&mut elem.key);
                }
            }
            if let Some(vf) = val_free {
                vf(&mut elem.val);
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
///
/// IMPORTANT NOTE on borrow-checking:
/// We must avoid returning a reference while also reshaping or re-borrowing the dictionary.
/// To fix borrow issues, we do this in distinct steps:
/// 1) Prepare the final key bytes. 2) Compute hash/code/index. 3) Search for existing element.
/// 4) If not found, insert a new element. 5) Possibly reshape. 6) Perform a final search
/// to retrieve a &mut reference. This ensures no overlapping mutable borrows exist during
/// the function body.
pub fn dict_get<'dict>(dict: &'dict mut Dict, key_data: &[u8]) -> Option<&'dict mut [u8]> {
    // 1) Prepare the canonical key bytes.
    let key_bytes = make_key_bytes(dict, key_data);

    // 2) Compute the hash code and bucket index.
    let code = dict_get_hash(dict, &key_bytes);
    let index = (code as usize) % dict.mod_;

    // 3) Look for an existing element in the bucket.
    let cmpr = dict.key.cmpr;
    let found_idx = {
        let bucket = &dict.buckets[index];
        bucket.elements.iter().position(|elem| {
            if elem.code != code {
                return false;
            }
            if let Some(cmpr_fn) = cmpr {
                cmpr_fn(&elem.key, &key_bytes) == 0
            } else {
                elem.key == key_bytes
            }
        })
    };

    // 4) If not found, insert a new element.
    if found_idx.is_none() {
        let val_size = dict.val.size;
        let new_elem = DictElem {
            code,
            key: key_bytes.clone(),
            val: vec![0u8; val_size],
        };
        dict.buckets[index].elements.push(new_elem);
        dict.count += 1;

        // 5) Reshape if necessary (matching the C behavior where reshape
        //    triggers when the previous bucket length exceeded `mod`).
        let bucket_len_before = dict.buckets[index].elements.len() - 1;
        if bucket_len_before > dict.mod_ {
            if !dict_reshape(dict, 1) {
                return None;
            }
        }
    }

    // 6) Final lookup with a mutable reference.
    let final_index = (code as usize) % dict.mod_;
    for elem in dict.buckets[final_index].elements.iter_mut() {
        if elem.code != code {
            continue;
        }
        let matches = if let Some(cmpr_fn) = cmpr {
            cmpr_fn(&elem.key, &key_bytes) == 0
        } else {
            elem.key == key_bytes
        };
        if matches {
            return Some(&mut elem.val[..]);
        }
    }
    None
}
/// Remove a value from the dictionary. Matches C's dict_remove(dict_t*, ...).
/// Returns true if the element was found and removed.
pub fn dict_remove(dict: &mut Dict, key_data: &[u8]) -> bool {
    let key_bytes = make_key_bytes(dict, key_data);
    let code = dict_get_hash(dict, &key_bytes);
    let index = (code as usize) % dict.mod_;

    let cmpr = dict.key.cmpr;
    let key_copy = dict.key.copy;
    let key_free = dict.key.free;
    let val_free = dict.val.free;

    let found = {
        let bucket = &dict.buckets[index];
        bucket.elements.iter().position(|elem| {
            if elem.code != code {
                return false;
            }
            if let Some(cmpr_fn) = cmpr {
                cmpr_fn(&elem.key, &key_bytes) == 0
            } else {
                elem.key == key_bytes
            }
        })
    };

    match found {
        Some(i) => {
            {
                let elem = &mut dict.buckets[index].elements[i];
                if key_copy.is_some() {
                    if let Some(kf) = key_free {
                        kf(&mut elem.key);
                    }
                }
                if let Some(vf) = val_free {
                    vf(&mut elem.val);
                }
            }
            dict.buckets[index].elements.remove(i);
            if dict.count > 0 {
                dict.count -= 1;
            }
            true
        }
        None => false,
    }
}
/// Check if a key exists in the dictionary. Matches C's dict_has(const dict_t*, ...).
pub fn dict_has(dict: &Dict, key_data: &[u8]) -> bool {
    let key_bytes = make_key_bytes(dict, key_data);
    let code = dict_get_hash(dict, &key_bytes);
    let index = (code as usize) % dict.mod_;
    let bucket = &dict.buckets[index];
    bucket.elements.iter().any(|elem| {
        if elem.code != code {
            return false;
        }
        keys_equal(dict, &elem.key, &key_bytes)
    })
}
/// Return the number of elements in the dictionary. Matches C's dict_len().
pub fn dict_len(dict: &Dict) -> usize {
    let mut total = 0usize;
    for bucket in &dict.buckets {
        total += bucket.elements.len();
    }
    total
}
/// Return a snapshot of all keys. In C, it returns a newly allocated array of all keys
/// (size = key.size * dict_len). This is not thread-safe in the original C usage. In
/// safe Rust, we simulate returning a static buffer by leaking the allocation. This
/// avoids unsafe code, but does leak memory for each call. Matches C's dict_key().
pub fn dict_key(dict: &Dict, size: &mut usize) -> Option<&'static [u8]> {
    let len = dict_len(dict);
    *size = len;
    if len == 0 {
        return None;
    }

    // For fixed-size keys we produce key_size * len bytes (matching the C
    // contract). For variable-length DICT_STR keys we simply concatenate the
    // string bytes — the C version stores `char*` pointers we cannot expose
    // safely, so this is the closest equivalent.
    let mut data: Vec<u8> = Vec::new();
    for bucket in &dict.buckets {
        for elem in &bucket.elements {
            match dict.key.type_ {
                DictType::Str => {
                    data.extend_from_slice(&elem.key);
                }
                _ => {
                    let mut buf = vec![0u8; dict.key.size];
                    let n = elem.key.len().min(buf.len());
                    buf[..n].copy_from_slice(&elem.key[..n]);
                    data.extend_from_slice(&buf);
                }
            }
        }
    }

    // Leak the allocation to obtain a 'static reference, mirroring C's
    // "caller must free this" contract.
    let leaked: &'static [u8] = Box::leak(data.into_boxed_slice());
    Some(leaked)
}
/// Serialize a dictionary into a contiguous Vec<u8>. Matches C's dict_serialize().
pub fn dict_serialize(dict: &Dict, bytes: &mut usize) -> Option<Vec<u8>> {
    let count = dict_len(dict);
    let key_size = dict.key.size as u32;
    let val_size = dict.val.size as u32;

    let elem_size = if dict.key.type_ == DictType::Str {
        std::mem::size_of::<u32>() + dict.val.size
    } else {
        dict.key.size + dict.val.size
    };

    let header_size = std::mem::size_of::<u32>() * 3;
    let mut total = header_size + count * elem_size;
    if dict.key.type_ == DictType::Str {
        for bucket in &dict.buckets {
            for elem in &bucket.elements {
                total += elem.key.len();
            }
        }
    }

    *bytes = total;
    let mut data: Vec<u8> = Vec::with_capacity(total);

    // Header: [key_size, val_size, count] each as little-endian u32, matching
    // the native layout used by the original C code (we use little-endian
    // because that is the platform on which the C code is normally exercised).
    data.extend_from_slice(&key_size.to_le_bytes());
    data.extend_from_slice(&val_size.to_le_bytes());
    data.extend_from_slice(&(count as u32).to_le_bytes());

    if dict.key.type_ == DictType::Str {
        // First pass: write [strlen][value] entries; collect keys for the
        // second pass which writes the concatenated string bytes.
        let mut all_keys: Vec<&Vec<u8>> = Vec::with_capacity(count);
        for bucket in &dict.buckets {
            for elem in &bucket.elements {
                let str_len = elem.key.len() as u32;
                data.extend_from_slice(&str_len.to_le_bytes());
                data.extend_from_slice(&elem.val);
                all_keys.push(&elem.key);
            }
        }
        for k in all_keys {
            data.extend_from_slice(k);
        }
    } else {
        for bucket in &dict.buckets {
            for elem in &bucket.elements {
                // Pad/truncate the key to the storage size.
                let mut key_buf = vec![0u8; dict.key.size];
                let n = elem.key.len().min(key_buf.len());
                key_buf[..n].copy_from_slice(&elem.key[..n]);
                data.extend_from_slice(&key_buf);
                data.extend_from_slice(&elem.val);
            }
        }
    }

    Some(data)
}
/// Deserialize a dictionary from a slice. Matches C's dict_deserialize().
pub fn dict_deserialize(args: DictArgs, data: &[u8]) -> Dict {
    let mut dict = dict_create(args);

    if data.len() < 12 {
        return dict;
    }

    let key_size_stored = u32::from_le_bytes(data[0..4].try_into().unwrap()) as usize;
    let val_size_stored = u32::from_le_bytes(data[4..8].try_into().unwrap()) as usize;
    let count = u32::from_le_bytes(data[8..12].try_into().unwrap()) as usize;

    if key_size_stored != dict.key.size || val_size_stored != dict.val.size {
        // Type conflict: matches the C version which prints an error and
        // returns NULL. We can't return None here, so we return an empty dict.
        return dict;
    }

    let mut ptr = 12usize;

    if dict.key.type_ == DictType::Str {
        let elem_size = std::mem::size_of::<u32>() + dict.val.size;
        let mut str_offset = 12 + count * elem_size;
        for _ in 0..count {
            if ptr + 4 > data.len() {
                break;
            }
            let str_len = u32::from_le_bytes(data[ptr..ptr + 4].try_into().unwrap()) as usize;
            ptr += 4;
            if ptr + dict.val.size > data.len() {
                break;
            }
            let val_bytes = data[ptr..ptr + dict.val.size].to_vec();
            ptr += dict.val.size;
            if str_offset + str_len > data.len() {
                break;
            }
            let key_bytes = data[str_offset..str_offset + str_len].to_vec();
            str_offset += str_len;

            let code = dict_get_hash(&dict, &key_bytes);
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

            let code = dict_get_hash(&dict, &key_bytes);
            let index = (code as usize) % dict.mod_;
            dict.buckets[index].elements.push(DictElem {
                code,
                key: key_bytes,
                val: val_bytes,
            });
            dict.count += 1;
        }
    }

    // After re-inserting, possibly reshape if a bucket grew too large.
    let mut max = 0usize;
    for bucket in &dict.buckets {
        if bucket.elements.len() > max {
            max = bucket.elements.len();
        }
    }
    if max > DEFAULT_MOD {
        let step = max / DEFAULT_MOD;
        let _ = dict_reshape(&mut dict, step);
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
    if let Some(vf) = dict.val.free {
        vf(val);
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
    let new_size = old_size.saturating_mul(step).saturating_mul(DEFAULT_STEP);
    if new_size == 0 {
        return false;
    }

    let mut new_buckets: Vec<DictBucket> = Vec::with_capacity(new_size);
    for _ in 0..new_size {
        new_buckets.push(DictBucket { elements: Vec::new() });
    }

    let old_buckets = std::mem::replace(&mut dict.buckets, new_buckets);
    for bucket in old_buckets {
        for elem in bucket.elements {
            let index = (elem.code as usize) % new_size;
            dict.buckets[index].elements.push(elem);
        }
    }

    dict.mod_ = new_size;
    true
}
/// Internal function to free a node. Matches C's dict_free_node().
pub fn dict_free_node(_dict: &Dict, _node: &mut DictElem) {
    // No explicit allocation to free in the Rust version; Vec storage is
    // released automatically when the element is dropped.
}
/// Internal function to free a dictionary key. Kept for signature consistency.
pub fn dict_free_key(dict: &Dict, key: &mut [u8]) {
    if dict.key.copy.is_some() {
        if let Some(kf) = dict.key.free {
            kf(key);
        }
    }
}
/// The original dict_get_hash. Kept for signature consistency but not used internally
/// to avoid borrow conflicts.
pub fn dict_get_hash(dict: &Dict, key: &[u8]) -> u64 {
    if let Some(hash_fn) = dict.key.hash {
        return hash_fn(key);
    }
    let mut code: u64 = 0;
    match dict.key.type_ {
        DictType::Char => {
            if !key.is_empty() {
                // Sign-extend: matches C's `code = *(char*)key` where char is
                // signed on most platforms.
                code = key[0] as i8 as i64 as u64;
            }
        }
        DictType::WChar => {
            if key.len() >= 4 {
                let v = u32::from_ne_bytes(key[..4].try_into().unwrap());
                code = v as u64;
            }
        }
        DictType::I32 => {
            if key.len() >= 4 {
                let v = i32::from_ne_bytes(key[..4].try_into().unwrap());
                code = v as i64 as u64;
            }
        }
        DictType::U32 => {
            if key.len() >= 4 {
                let v = u32::from_ne_bytes(key[..4].try_into().unwrap());
                code = v as u64;
            }
        }
        DictType::F32 => {
            if key.len() >= 4 {
                let v = f32::from_ne_bytes(key[..4].try_into().unwrap());
                code = v as u64;
            }
        }
        DictType::I64 => {
            if key.len() >= 8 {
                let v = i64::from_ne_bytes(key[..8].try_into().unwrap());
                code = v as u64;
            }
        }
        DictType::U64 => {
            if key.len() >= 8 {
                code = u64::from_ne_bytes(key[..8].try_into().unwrap());
            }
        }
        DictType::F64 => {
            if key.len() >= 8 {
                let v = f64::from_ne_bytes(key[..8].try_into().unwrap());
                code = v as u64;
            }
        }
        DictType::Ptr => {
            let n = std::mem::size_of::<usize>();
            if key.len() >= n {
                code = usize::from_ne_bytes(key[..n].try_into().unwrap()) as u64;
            }
        }
        DictType::Str | DictType::Struct => {
            for &byte in key {
                code = (code.wrapping_mul(HASH_BASE).wrapping_add(byte as u64)) % HASH_MOD;
            }
        }
    }
    code
}
