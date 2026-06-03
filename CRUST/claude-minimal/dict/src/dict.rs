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

/// Compute the natural storage size for a key based on its type, mirroring the C
/// switch in dict_create. For DICT_STRUCT we round the user supplied size up to
/// the next pointer alignment.
fn compute_key_size(type_: DictType, raw_size: usize) -> usize {
    match type_ {
        DictType::Char => std::mem::size_of::<u8>(),
        DictType::WChar => std::mem::size_of::<u32>(),
        DictType::I32 => std::mem::size_of::<i32>(),
        DictType::U32 => std::mem::size_of::<u32>(),
        DictType::F32 => std::mem::size_of::<f32>(),
        DictType::I64 => std::mem::size_of::<i64>(),
        DictType::U64 => std::mem::size_of::<u64>(),
        DictType::F64 => std::mem::size_of::<f64>(),
        DictType::Ptr => std::mem::size_of::<*const u8>(),
        DictType::Str => std::mem::size_of::<*const u8>(),
        DictType::Struct => align_to_pointer(raw_size),
    }
}

/// Round `size` up to a multiple of `sizeof(uintptr_t)`, matching the C macro.
fn align_to_pointer(size: usize) -> usize {
    let align = std::mem::size_of::<usize>();
    (size + align - 1) & !(align - 1)
}

/// Normalize an externally supplied key slice to the canonical key bytes that
/// will be hashed/compared/stored.  For DICT_STR we keep the slice as-is (the
/// string content).  For all other types we copy into a buffer of exactly
/// `dict.key.size` bytes, padding with zeros if the input is shorter and
/// truncating if it is longer.
fn normalize_key(key_attr: &DictKeyAttr, key_data: &[u8]) -> Vec<u8> {
    match key_attr.type_ {
        DictType::Str => key_data.to_vec(),
        _ => {
            let target = if key_attr.size > 0 {
                key_attr.size
            } else {
                key_data.len()
            };
            let mut out = vec![0u8; target];
            let n = key_data.len().min(target);
            if n > 0 {
                out[..n].copy_from_slice(&key_data[..n]);
            }
            out
        }
    }
}

/// Compute the hash code of `key` according to the user supplied hash function
/// or the polynomial fallback used by the C implementation for STR/STRUCT.
fn compute_hash(key_attr: &DictKeyAttr, key: &[u8]) -> u64 {
    if let Some(h) = key_attr.hash {
        return h(key);
    }
    let mut code: u64 = 0;
    for &b in key {
        code = (code.wrapping_mul(HASH_BASE).wrapping_add(b as u64)) % HASH_MOD;
    }
    code
}

/// Compare two key slices for equality, using the user supplied comparator if
/// provided and falling back to byte-wise equality otherwise.
fn keys_equal(key_attr: &DictKeyAttr, a: &[u8], b: &[u8]) -> bool {
    if let Some(c) = key_attr.cmpr {
        return c(a, b) == 0;
    }
    a == b
}

/// Create a dictionary using detailed arguments. Matches C's dict_create().
pub fn dict_create(args: DictArgs) -> Dict {
    let mut key = args.key.clone();
    let key_size = compute_key_size(key.type_, key.size);
    key.size = key_size;

    let mut val = args.val.clone();
    val.size = align_to_pointer(val.size);

    let mod_ = DEFAULT_MOD;
    let buckets = (0..mod_)
        .map(|_| DictBucket { elements: Vec::new() })
        .collect();
    let key_temp = vec![0u8; key.size.max(1)];

    Dict {
        key,
        val,
        alloc: args.alloc,
        mod_,
        buckets,
        key_temp,
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
    for bucket in dict.buckets.iter_mut() {
        for elem in bucket.elements.iter_mut() {
            if let Some(kf) = key_free {
                kf(&mut elem.key);
            }
            if let Some(vf) = val_free {
                vf(&mut elem.val);
            }
        }
        bucket.elements.clear();
    }
    dict.buckets.clear();
    dict.count = 0;
    dict.mod_ = 0;
    dict.key_temp.clear();
    dict.keys_dump.clear();
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
    if dict.mod_ == 0 {
        // Re-initialize an empty dict (e.g. after destroy was called by mistake)
        dict.mod_ = DEFAULT_MOD;
        dict.buckets = (0..dict.mod_)
            .map(|_| DictBucket { elements: Vec::new() })
            .collect();
    }
    let key = normalize_key(&dict.key, key_data);
    let code = compute_hash(&dict.key, &key);

    // Phase 1: search for an existing entry
    let index = (code as usize) % dict.mod_;
    let mut found = false;
    for elem in dict.buckets[index].elements.iter() {
        if elem.code == code && keys_equal(&dict.key, &elem.key, &key) {
            found = true;
            break;
        }
    }

    // Phase 2: insert if missing, then reshape if the bucket grew too large
    if !found {
        let val_size = dict.val.size;
        let new_elem = DictElem {
            code,
            key: key.clone(),
            val: vec![0u8; val_size],
        };
        dict.buckets[index].elements.push(new_elem);
        dict.count += 1;

        if dict.buckets[index].elements.len() > dict.mod_ {
            dict_reshape(dict, 1);
        }
    }

    // Phase 3: re-resolve and return the value mutably
    let index = (code as usize) % dict.mod_;
    let mut idx_to_return: Option<usize> = None;
    for (i, elem) in dict.buckets[index].elements.iter().enumerate() {
        if elem.code == code && keys_equal(&dict.key, &elem.key, &key) {
            idx_to_return = Some(i);
            break;
        }
    }
    let i = idx_to_return?;
    Some(&mut dict.buckets[index].elements[i].val[..])
}
/// Remove a value from the dictionary. Matches C's dict_remove(dict_t*, ...).
/// Returns true if the element was found and removed.
pub fn dict_remove(dict: &mut Dict, key_data: &[u8]) -> bool {
    if dict.mod_ == 0 {
        return false;
    }
    let key = normalize_key(&dict.key, key_data);
    let code = compute_hash(&dict.key, &key);
    let index = (code as usize) % dict.mod_;

    let mut target: Option<usize> = None;
    for (i, elem) in dict.buckets[index].elements.iter().enumerate() {
        if elem.code == code && keys_equal(&dict.key, &elem.key, &key) {
            target = Some(i);
            break;
        }
    }

    if let Some(i) = target {
        let mut elem = dict.buckets[index].elements.remove(i);
        if let Some(kf) = dict.key.free {
            kf(&mut elem.key);
        }
        if let Some(vf) = dict.val.free {
            vf(&mut elem.val);
        }
        if dict.count > 0 {
            dict.count -= 1;
        }
        true
    } else {
        false
    }
}
/// Check if a key exists in the dictionary. Matches C's dict_has(const dict_t*, ...).
pub fn dict_has(dict: &Dict, key_data: &[u8]) -> bool {
    if dict.mod_ == 0 {
        return false;
    }
    let key = normalize_key(&dict.key, key_data);
    let code = compute_hash(&dict.key, &key);
    let index = (code as usize) % dict.mod_;
    for elem in dict.buckets[index].elements.iter() {
        if elem.code == code && keys_equal(&dict.key, &elem.key, &key) {
            return true;
        }
    }
    false
}
/// Return the number of elements in the dictionary. Matches C's dict_len().
pub fn dict_len(dict: &Dict) -> usize {
    dict.count
}
/// Return a snapshot of all keys. In C, it returns a newly allocated array of all keys
/// (size = key.size * dict_len). This is not thread-safe in the original C usage. In
/// safe Rust, we simulate returning a static buffer by leaking the allocation. This
/// avoids unsafe code, but does leak memory for each call. Matches C's dict_key().
pub fn dict_key(dict: &Dict, size: &mut usize) -> Option<&'static [u8]> {
    *size = dict.count;
    if *size == 0 {
        return None;
    }

    let key_size = dict.key.size;
    let mut buf: Vec<u8> = Vec::with_capacity(key_size * dict.count);
    let mut written: usize = 0;
    for bucket in dict.buckets.iter() {
        for elem in bucket.elements.iter() {
            // Pad/truncate the key to exactly key_size bytes for the array layout.
            let mut padded = vec![0u8; key_size];
            let n = elem.key.len().min(key_size);
            if n > 0 {
                padded[..n].copy_from_slice(&elem.key[..n]);
            }
            buf.extend_from_slice(&padded);
            written += 1;
            if written == *size {
                let leaked: &'static [u8] = Box::leak(buf.into_boxed_slice());
                return Some(leaked);
            }
        }
    }

    let leaked: &'static [u8] = Box::leak(buf.into_boxed_slice());
    Some(leaked)
}
/// Serialize a dictionary into a contiguous Vec<u8>. Matches C's dict_serialize().
pub fn dict_serialize(dict: &Dict, bytes: &mut usize) -> Option<Vec<u8>> {
    let mut data: Vec<u8> = Vec::new();
    let key_size = dict.key.size as u32;
    let val_size = dict.val.size as u32;
    let count = dict.count as u32;

    data.extend_from_slice(&key_size.to_le_bytes());
    data.extend_from_slice(&val_size.to_le_bytes());
    data.extend_from_slice(&count.to_le_bytes());

    for bucket in dict.buckets.iter() {
        for elem in bucket.elements.iter() {
            let klen = elem.key.len() as u32;
            data.extend_from_slice(&klen.to_le_bytes());
            data.extend_from_slice(&elem.key);
            // Make sure the value is exactly val.size bytes long.
            if elem.val.len() == dict.val.size {
                data.extend_from_slice(&elem.val);
            } else {
                let mut padded = vec![0u8; dict.val.size];
                let n = elem.val.len().min(dict.val.size);
                if n > 0 {
                    padded[..n].copy_from_slice(&elem.val[..n]);
                }
                data.extend_from_slice(&padded);
            }
        }
    }

    *bytes = data.len();
    Some(data)
}
/// Deserialize a dictionary from a slice. Matches C's dict_deserialize().
pub fn dict_deserialize(args: DictArgs, data: &[u8]) -> Dict {
    let mut dict = dict_create(args);

    if data.len() < 12 {
        return dict;
    }

    let mut hdr = [0u8; 4];
    hdr.copy_from_slice(&data[0..4]);
    let key_size = u32::from_le_bytes(hdr) as usize;
    hdr.copy_from_slice(&data[4..8]);
    let val_size = u32::from_le_bytes(hdr) as usize;
    hdr.copy_from_slice(&data[8..12]);
    let count = u32::from_le_bytes(hdr) as usize;

    if key_size != dict.key.size || val_size != dict.val.size {
        // Type/size mismatch -> corrupt or wrong dict args.
        return dict;
    }

    let mut ptr: usize = 12;
    for _ in 0..count {
        if ptr + 4 > data.len() {
            break;
        }
        hdr.copy_from_slice(&data[ptr..ptr + 4]);
        let klen = u32::from_le_bytes(hdr) as usize;
        ptr += 4;
        if ptr + klen + val_size > data.len() {
            break;
        }
        let key = data[ptr..ptr + klen].to_vec();
        ptr += klen;
        let val = data[ptr..ptr + val_size].to_vec();
        ptr += val_size;

        let code = compute_hash(&dict.key, &key);
        let index = (code as usize) % dict.mod_;
        dict.buckets[index].elements.push(DictElem { code, key, val });
        dict.count += 1;
    }

    // Mirror the post-deserialize reshape logic of the C implementation.
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
/// Convenience function to create a dictionary using inline arguments, mirroring the
/// C macro dict_create_args(...).
pub fn dict_create_args(args: DictArgs) -> Dict {
    dict_create(args)
}
/// The original dict_key_equals. Kept for signature consistency but not used internally
/// to avoid borrow conflicts.
pub fn dict_key_equals(dict: &Dict, a: &[u8], b: &[u8]) -> bool {
    keys_equal(&dict.key, a, b)
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
    let factor = step.max(1);
    let new_size = dict.mod_.max(1) * factor * DEFAULT_STEP;
    if new_size == 0 {
        return false;
    }

    let mut new_buckets: Vec<DictBucket> = (0..new_size)
        .map(|_| DictBucket { elements: Vec::new() })
        .collect();

    let old_buckets = std::mem::take(&mut dict.buckets);
    for bucket in old_buckets {
        for elem in bucket.elements {
            let index = (elem.code as usize) % new_size;
            new_buckets[index].elements.push(elem);
        }
    }

    dict.mod_ = new_size;
    dict.buckets = new_buckets;
    true
}
/// Internal function to free a node. Matches C's dict_free_node().
pub fn dict_free_node(_dict: &Dict, _node: &mut DictElem) {
    // No-op in safe Rust. The Vec inside DictElem will be dropped automatically.
}
/// Internal function to free a dictionary key. Kept for signature consistency.
pub fn dict_free_key(dict: &Dict, key: &mut [u8]) {
    if let Some(kf) = dict.key.free {
        kf(key);
    }
}
/// The original dict_get_hash. Kept for signature consistency but not used internally
/// to avoid borrow conflicts.
pub fn dict_get_hash(dict: &Dict, key: &[u8]) -> u64 {
    compute_hash(&dict.key, key)
}

// Suppress unused-import warnings for items that come with the original module
// scaffolding but aren't exercised in this safe Rust port.
#[allow(dead_code)]
fn _suppress_unused() {
    let _: Option<Ordering> = None;
    let _ = DefaultHasher::new();
    let _: Option<Mutex<()>> = None;
    fn _hash_check<T: StdHash + Hasher>() {}
}
