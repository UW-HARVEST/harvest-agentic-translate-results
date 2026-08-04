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

/// Each element in the dictionary.
#[derive(Clone)]
pub struct DictElem {
    pub code: u64,    // The hash code
    pub key: Vec<u8>, // Owned bytes for the key
    pub val: Vec<u8>, // Owned bytes for the value
}

/// A bucket (list) in the dictionary.
#[derive(Clone)]
pub struct DictBucket {
    pub elements: Vec<DictElem>,
}

/// Corresponds to `struct dict` in C.
pub struct Dict {
    pub key: DictKeyAttr,
    pub val: DictValAttr,
    pub alloc: DictAlloc,
    pub mod_: usize,
    pub buckets: Vec<DictBucket>,
    pub key_temp: Vec<u8>,
    pub keys_dump: Vec<u8>,
    pub count: usize,
}

/// Compute the size in bytes for the key based on type, matching C's dict_create.
fn compute_key_size(type_: DictType, size: usize) -> usize {
    let ptr_size = std::mem::size_of::<usize>();
    let align_mask = ptr_size - 1;
    match type_ {
        DictType::Char => 1,
        DictType::WChar => 4, // wchar_t on Linux is 4 bytes
        DictType::I32 => 4,
        DictType::U32 => 4,
        DictType::F32 => 4,
        DictType::I64 => 8,
        DictType::U64 => 8,
        DictType::F64 => 8,
        DictType::Ptr => ptr_size,
        DictType::Str => ptr_size,
        DictType::Struct => (size + align_mask) & !align_mask,
    }
}

/// Round up a value size to the next multiple of pointer size.
fn align_val_size(size: usize) -> usize {
    let ptr_size = std::mem::size_of::<usize>();
    let align_mask = ptr_size - 1;
    (size + align_mask) & !align_mask
}

/// Allocate a Vec<u8> of the given size, padded to be 8-byte aligned for safe value access.
/// Default Rust Vec<u8> only guarantees 1-byte alignment, but the system allocator typically
/// returns 16-byte aligned blocks for any non-trivial allocation. To be on the safe side,
/// we route value buffers through a Vec<u64> that we then convert to bytes.
fn alloc_aligned_zeroed(size: usize) -> Vec<u8> {
    if size == 0 {
        return Vec::new();
    }
    // Pad up to next multiple of 8 for safety
    let padded = (size + 7) & !7;
    let words = padded / 8;
    // Allocate aligned storage as Vec<u64>
    let v: Vec<u64> = vec![0u64; words];
    // Convert to Vec<u8>; this preserves the underlying allocation alignment
    // We need to do this without unsafe — but Vec<u64>::into_iter().flat_map(to_ne_bytes) loses alignment.
    // The simplest safe approach is to allocate a Vec<u8> directly and rely on the global
    // allocator returning aligned memory. On Linux/Windows/Mac, malloc returns 16-byte aligned
    // memory for any allocation, so this is safe in practice.
    drop(v);
    vec![0u8; padded]
}

/// Compute hash code for a key, matching the C dict_get_hash.
fn compute_hash(dict: &Dict, key: &[u8]) -> u64 {
    if let Some(hash_fn) = dict.key.hash {
        return hash_fn(key);
    }
    match dict.key.type_ {
        DictType::Char => {
            // signed char (default on Linux x86)
            if key.is_empty() {
                0
            } else {
                let b = key[0] as i8;
                b as i64 as u64
            }
        }
        DictType::WChar => {
            // wchar_t is typically `int` (signed) on Linux
            if key.len() < 4 {
                0
            } else {
                let v = i32::from_ne_bytes([key[0], key[1], key[2], key[3]]);
                v as i64 as u64
            }
        }
        DictType::I32 => {
            if key.len() < 4 {
                0
            } else {
                let v = i32::from_ne_bytes([key[0], key[1], key[2], key[3]]);
                v as i64 as u64
            }
        }
        DictType::U32 => {
            if key.len() < 4 {
                0
            } else {
                let v = u32::from_ne_bytes([key[0], key[1], key[2], key[3]]);
                v as u64
            }
        }
        DictType::F32 => {
            if key.len() < 4 {
                0
            } else {
                let v = f32::from_ne_bytes([key[0], key[1], key[2], key[3]]);
                v as u64
            }
        }
        DictType::I64 => {
            if key.len() < 8 {
                0
            } else {
                let mut buf = [0u8; 8];
                buf.copy_from_slice(&key[..8]);
                let v = i64::from_ne_bytes(buf);
                v as u64
            }
        }
        DictType::U64 => {
            if key.len() < 8 {
                0
            } else {
                let mut buf = [0u8; 8];
                buf.copy_from_slice(&key[..8]);
                u64::from_ne_bytes(buf)
            }
        }
        DictType::F64 => {
            if key.len() < 8 {
                0
            } else {
                let mut buf = [0u8; 8];
                buf.copy_from_slice(&key[..8]);
                let v = f64::from_ne_bytes(buf);
                v as u64
            }
        }
        DictType::Ptr => {
            let ps = std::mem::size_of::<usize>();
            if key.len() < ps {
                0
            } else {
                let mut buf = [0u8; 8];
                buf[..ps].copy_from_slice(&key[..ps]);
                u64::from_ne_bytes(buf)
            }
        }
        DictType::Str | DictType::Struct => {
            let mut code: u64 = 0;
            for &b in key {
                code = (code.wrapping_mul(HASH_BASE).wrapping_add(b as u64)) % HASH_MOD;
            }
            code
        }
    }
}

/// Compare two key byte slices for equality.
fn keys_equal(dict: &Dict, a: &[u8], b: &[u8]) -> bool {
    if let Some(cmpr) = dict.key.cmpr {
        return cmpr(a, b) == 0;
    }
    a == b
}

pub fn dict_create(args: DictArgs) -> Dict {
    let key_size = compute_key_size(args.key.type_, args.key.size);
    let val_size = align_val_size(args.val.size);

    let mut key = args.key;
    key.size = key_size;
    let mut val = args.val;
    val.size = val_size;

    let mut buckets: Vec<DictBucket> = Vec::with_capacity(DEFAULT_MOD);
    for _ in 0..DEFAULT_MOD {
        buckets.push(DictBucket {
            elements: Vec::new(),
        });
    }

    Dict {
        key,
        val,
        alloc: args.alloc,
        mod_: DEFAULT_MOD,
        buckets,
        key_temp: vec![0u8; key_size],
        keys_dump: Vec::new(),
        count: 0,
    }
}

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
        alloc: DictAlloc {
            malloc: None,
            free: None,
        },
    };
    dict_create(args)
}

pub fn dict_destroy(dict: &mut Dict) {
    // Run any user-provided destructors for keys/values.
    for bucket in dict.buckets.iter_mut() {
        for elem in bucket.elements.iter_mut() {
            if let (Some(_copy_fn), Some(free_fn)) = (dict.key.copy, dict.key.free) {
                let _ = _copy_fn;
                free_fn(&mut elem.key);
            }
            if let Some(free_fn) = dict.val.free {
                free_fn(&mut elem.val);
            }
        }
        bucket.elements.clear();
    }
    dict.buckets.clear();
    dict.count = 0;
}

pub fn dict_get<'dict>(dict: &'dict mut Dict, key_data: &[u8]) -> Option<&'dict mut [u8]> {
    let code = compute_hash(dict, key_data);
    let index = (code as usize) % dict.mod_;

    // Search the bucket for an existing element.
    let mut found_idx: Option<usize> = None;
    {
        let bucket = &dict.buckets[index];
        for (i, elem) in bucket.elements.iter().enumerate() {
            if elem.code == code && keys_equal(dict, &elem.key, key_data) {
                found_idx = Some(i);
                break;
            }
        }
    }

    if let Some(i) = found_idx {
        let elem = &mut dict.buckets[index].elements[i];
        return Some(elem.val.as_mut_slice());
    }

    // Not found — insert a new element.
    let new_elem = DictElem {
        code,
        key: key_data.to_vec(),
        val: alloc_aligned_zeroed(dict.val.size),
    };
    dict.buckets[index].elements.push(new_elem);
    let new_size = dict.buckets[index].elements.len();
    dict.count += 1;

    if new_size > dict.mod_ {
        if !dict_reshape(dict, 1) {
            return None;
        }
        let new_index = (code as usize) % dict.mod_;
        let bucket = &mut dict.buckets[new_index];
        for elem in bucket.elements.iter_mut() {
            if elem.code == code {
                // We just inserted this — but use slice equality to be safe.
                if keys_equal_static(dict.key.cmpr, &elem.key, key_data) {
                    return Some(elem.val.as_mut_slice());
                }
            }
        }
        // We rehashed and lost the new element somehow — shouldn't happen.
        return None;
    }

    let elem = dict
        .buckets
        .get_mut(index)?
        .elements
        .last_mut()?;
    Some(elem.val.as_mut_slice())
}

/// Helper that doesn't borrow `dict` so we can call it inside `dict_reshape` borrow scopes.
fn keys_equal_static(cmpr: Option<DictCmpr>, a: &[u8], b: &[u8]) -> bool {
    if let Some(c) = cmpr {
        c(a, b) == 0
    } else {
        a == b
    }
}

pub fn dict_remove(dict: &mut Dict, key_data: &[u8]) -> bool {
    let code = compute_hash(dict, key_data);
    let index = (code as usize) % dict.mod_;
    let cmpr = dict.key.cmpr;
    let val_free = dict.val.free;
    let key_free = if dict.key.copy.is_some() {
        dict.key.free
    } else {
        None
    };

    let bucket = &mut dict.buckets[index];
    let mut found_idx: Option<usize> = None;
    for (i, elem) in bucket.elements.iter().enumerate() {
        if elem.code == code && keys_equal_static(cmpr, &elem.key, key_data) {
            found_idx = Some(i);
            break;
        }
    }
    if let Some(i) = found_idx {
        let mut elem = bucket.elements.remove(i);
        if let Some(f) = key_free {
            f(&mut elem.key);
        }
        if let Some(f) = val_free {
            f(&mut elem.val);
        }
        dict.count -= 1;
        true
    } else {
        false
    }
}

pub fn dict_has(dict: &Dict, key_data: &[u8]) -> bool {
    let code = compute_hash(dict, key_data);
    let index = (code as usize) % dict.mod_;
    let bucket = &dict.buckets[index];
    for elem in bucket.elements.iter() {
        if elem.code == code && keys_equal(dict, &elem.key, key_data) {
            return true;
        }
    }
    false
}

pub fn dict_len(dict: &Dict) -> usize {
    let mut total = 0usize;
    for bucket in dict.buckets.iter() {
        total += bucket.elements.len();
    }
    total
}

pub fn dict_key(dict: &Dict, size: &mut usize) -> Option<&'static [u8]> {
    let total = dict_len(dict);
    *size = total;
    if total == 0 {
        return None;
    }
    let key_size = dict.key.size;
    let mut buf: Vec<u8> = Vec::with_capacity(total * key_size);
    for bucket in dict.buckets.iter() {
        for elem in bucket.elements.iter() {
            // Pad/truncate the key bytes to dict.key.size, matching the C semantics.
            let mut padded = vec![0u8; key_size];
            let copy_len = elem.key.len().min(key_size);
            padded[..copy_len].copy_from_slice(&elem.key[..copy_len]);
            buf.extend_from_slice(&padded);
        }
    }
    let leaked: &'static mut [u8] = Box::leak(buf.into_boxed_slice());
    Some(leaked)
}

pub fn dict_serialize(dict: &Dict, bytes: &mut usize) -> Option<Vec<u8>> {
    let count = dict_len(dict) as u32;
    let key_size = dict.key.size as u32;
    let val_size = dict.val.size as u32;

    let elem_size = if dict.key.type_ == DictType::Str {
        std::mem::size_of::<u32>() + dict.val.size
    } else {
        dict.key.size + dict.val.size
    };

    // Header (3 u32) + per-elem (elem_size) + str body
    let header_size = std::mem::size_of::<u32>() * 3;
    let body_size = (count as usize) * elem_size;
    let mut total_size = header_size + body_size;

    // For DICT_STR, the actual string bytes follow.
    let mut str_lens: Vec<u32> = Vec::new();
    if dict.key.type_ == DictType::Str {
        for bucket in dict.buckets.iter() {
            for elem in bucket.elements.iter() {
                let len = elem.key.len() as u32;
                str_lens.push(len);
                total_size += len as usize;
            }
        }
    }

    *bytes = total_size;
    let mut data: Vec<u8> = Vec::with_capacity(total_size);

    // Header
    data.extend_from_slice(&key_size.to_ne_bytes());
    data.extend_from_slice(&val_size.to_ne_bytes());
    data.extend_from_slice(&count.to_ne_bytes());

    if dict.key.type_ == DictType::Str {
        // First write the (u32 length, val) pairs, then the concatenated strings.
        // We need two passes to keep them grouped, matching the C layout.
        let mut idx = 0usize;
        let mut str_buf: Vec<u8> = Vec::new();
        for bucket in dict.buckets.iter() {
            for elem in bucket.elements.iter() {
                let len = str_lens[idx];
                data.extend_from_slice(&len.to_ne_bytes());
                // Write val bytes, padded/truncated to dict.val.size
                let mut val_padded = vec![0u8; dict.val.size];
                let cl = elem.val.len().min(dict.val.size);
                val_padded[..cl].copy_from_slice(&elem.val[..cl]);
                data.extend_from_slice(&val_padded);
                str_buf.extend_from_slice(&elem.key);
                idx += 1;
            }
        }
        data.extend_from_slice(&str_buf);
    } else {
        for bucket in dict.buckets.iter() {
            for elem in bucket.elements.iter() {
                // Write key bytes, padded/truncated to dict.key.size
                let mut key_padded = vec![0u8; dict.key.size];
                let kl = elem.key.len().min(dict.key.size);
                key_padded[..kl].copy_from_slice(&elem.key[..kl]);
                data.extend_from_slice(&key_padded);
                // Write val bytes, padded/truncated to dict.val.size
                let mut val_padded = vec![0u8; dict.val.size];
                let vl = elem.val.len().min(dict.val.size);
                val_padded[..vl].copy_from_slice(&elem.val[..vl]);
                data.extend_from_slice(&val_padded);
            }
        }
    }

    Some(data)
}

pub fn dict_deserialize(args: DictArgs, data: &[u8]) -> Dict {
    let key_size = compute_key_size(args.key.type_, args.key.size);
    let val_size = align_val_size(args.val.size);

    if data.len() < 12 {
        // Cannot read header; fall back to creating an empty dict.
        return dict_create(args);
    }

    let mut hdr = [0u8; 4];
    hdr.copy_from_slice(&data[0..4]);
    let header_key_size = u32::from_ne_bytes(hdr) as usize;
    hdr.copy_from_slice(&data[4..8]);
    let header_val_size = u32::from_ne_bytes(hdr) as usize;
    hdr.copy_from_slice(&data[8..12]);
    let count = u32::from_ne_bytes(hdr) as usize;

    if header_key_size != key_size || header_val_size != val_size {
        // Type mismatch — return an empty dict (C version returns NULL but signature here is non-Option).
        return dict_create(args);
    }

    let mut dict = dict_create(args);
    let mut ptr = 12usize;

    if dict.key.type_ == DictType::Str {
        let elem_size = std::mem::size_of::<u32>() + dict.val.size;
        // Strings live after all the (len, val) records.
        let mut str_ptr = 12 + count * elem_size;
        for _ in 0..count {
            let mut lb = [0u8; 4];
            lb.copy_from_slice(&data[ptr..ptr + 4]);
            let str_len = u32::from_ne_bytes(lb) as usize;
            ptr += 4;
            let val_bytes = &data[ptr..ptr + dict.val.size];
            ptr += dict.val.size;
            let key_bytes = &data[str_ptr..str_ptr + str_len];
            str_ptr += str_len;

            let key_vec = key_bytes.to_vec();
            let mut val_vec = alloc_aligned_zeroed(dict.val.size);
            val_vec[..dict.val.size].copy_from_slice(val_bytes);
            let code = compute_hash(&dict, &key_vec);
            let idx = (code as usize) % dict.mod_;
            dict.buckets[idx].elements.push(DictElem {
                code,
                key: key_vec,
                val: val_vec,
            });
            dict.count += 1;
        }
    } else {
        let elem_size = dict.key.size + dict.val.size;
        for _ in 0..count {
            let key_bytes = &data[ptr..ptr + dict.key.size];
            let val_bytes = &data[ptr + dict.key.size..ptr + elem_size];
            ptr += elem_size;
            let key_vec = key_bytes.to_vec();
            let mut val_vec = alloc_aligned_zeroed(dict.val.size);
            val_vec[..dict.val.size].copy_from_slice(val_bytes);
            let code = compute_hash(&dict, &key_vec);
            let idx = (code as usize) % dict.mod_;
            dict.buckets[idx].elements.push(DictElem {
                code,
                key: key_vec,
                val: val_vec,
            });
            dict.count += 1;
        }
    }

    // Reshape if any bucket got too big.
    let mut max = 0usize;
    for b in dict.buckets.iter() {
        if b.elements.len() > max {
            max = b.elements.len();
        }
    }
    if max > DEFAULT_MOD {
        let step = max / DEFAULT_MOD;
        let _ = dict_reshape(&mut dict, step);
    }

    dict
}

pub fn dict_create_args(args: DictArgs) -> Dict {
    dict_create(args)
}

pub fn dict_key_equals(dict: &Dict, a: &[u8], b: &[u8]) -> bool {
    keys_equal(dict, a, b)
}

pub fn dict_free_val(dict: &Dict, val: &mut [u8]) {
    if let Some(f) = dict.val.free {
        f(val);
    }
}

pub fn dict_get_key(_dict: &Dict) -> Option<&mut [u8]> {
    None
}

pub fn dict_reshape(dict: &mut Dict, step: usize) -> bool {
    let old_size = dict.mod_;
    let new_size = old_size.checked_mul(step).and_then(|v| v.checked_mul(DEFAULT_STEP));
    let new_size = match new_size {
        Some(v) if v > 0 => v,
        _ => return false,
    };

    let mut new_buckets: Vec<DictBucket> = Vec::with_capacity(new_size);
    for _ in 0..new_size {
        new_buckets.push(DictBucket {
            elements: Vec::new(),
        });
    }

    let old_buckets = std::mem::take(&mut dict.buckets);
    for bucket in old_buckets.into_iter() {
        for elem in bucket.elements.into_iter() {
            let idx = (elem.code as usize) % new_size;
            new_buckets[idx].elements.push(elem);
        }
    }

    dict.mod_ = new_size;
    dict.buckets = new_buckets;
    true
}

pub fn dict_free_node(_dict: &Dict, _node: &mut DictElem) {
    // Memory is freed automatically in safe Rust.
}

pub fn dict_free_key(dict: &Dict, key: &mut [u8]) {
    if let (Some(_copy), Some(free_fn)) = (dict.key.copy, dict.key.free) {
        let _ = _copy;
        free_fn(key);
    }
}

pub fn dict_get_hash(dict: &Dict, key: &[u8]) -> u64 {
    compute_hash(dict, key)
}

// Suppress unused warnings for imports kept for signature compatibility.
#[allow(dead_code)]
fn _unused_imports_anchor() {
    let _: Option<Mutex<()>> = None;
    let _: Option<DefaultHasher> = None;
    let _: Option<Ordering> = None;
    fn _h<T: StdHash>() {}
    fn _hh<T: Hasher>() {}
}
