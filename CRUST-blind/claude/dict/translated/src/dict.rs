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

/// The conventional pointer size used by the original C code (sizeof(uintptr_t)).
fn ptr_size() -> usize {
    std::mem::size_of::<usize>()
}

/// Round `size` up to a multiple of `align`.
fn align_up(size: usize, align: usize) -> usize {
    if align == 0 {
        return size;
    }
    (size + align - 1) & !(align - 1)
}

/// Resolve the byte size of a key based on its type. For `Struct`, the
/// caller-provided size is rounded up to a pointer-aligned boundary, mirroring
/// the C implementation.
fn type_key_size(t: DictType, struct_size: usize) -> usize {
    match t {
        DictType::Char => 1,
        DictType::WChar => 4, // wchar_t is 4 bytes on Linux
        DictType::I32 | DictType::U32 | DictType::F32 => 4,
        DictType::I64 | DictType::U64 | DictType::F64 => 8,
        DictType::Ptr | DictType::Str => ptr_size(),
        DictType::Struct => align_up(struct_size, ptr_size()),
    }
}

/// Build a fresh empty dictionary from `args`, computing the proper key/val
/// sizes and creating an empty bucket array of `DEFAULT_MOD` buckets.
fn build_dict(args: DictArgs) -> Dict {
    let key_size = type_key_size(args.key.type_, args.key.size);
    let val_size = align_up(args.val.size, ptr_size());

    let mut key_attr = args.key.clone();
    key_attr.size = key_size;

    let mut val_attr = args.val.clone();
    val_attr.size = val_size;

    let mod_ = DEFAULT_MOD;
    let buckets = (0..mod_)
        .map(|_| DictBucket {
            elements: Vec::new(),
        })
        .collect::<Vec<_>>();

    Dict {
        key: key_attr,
        val: val_attr,
        alloc: args.alloc,
        mod_,
        buckets,
        key_temp: vec![0u8; key_size],
        keys_dump: Vec::new(),
        count: 0,
    }
}

/// Convert raw user-supplied key bytes into the canonical key representation
/// stored inside a `DictElem`.
///
/// * For `Str`, the input is returned as-is (variable length string bytes).
/// * For all other types, the input is truncated/zero-padded to exactly
///   `dict.key.size` bytes.
fn prepare_key_bytes(dict: &Dict, key_data: &[u8]) -> Vec<u8> {
    if dict.key.type_ == DictType::Str {
        return key_data.to_vec();
    }
    let mut buf = vec![0u8; dict.key.size];
    let n = key_data.len().min(dict.key.size);
    if n > 0 {
        buf[..n].copy_from_slice(&key_data[..n]);
    }
    buf
}

/// Read the first 4 bytes of `bytes` as a little-endian `u32`.
fn read_u32(bytes: &[u8]) -> u32 {
    u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]])
}

/// Compute the hash code for a prepared key.
fn compute_hash_internal(key_attr: &DictKeyAttr, key: &[u8]) -> u64 {
    if let Some(h) = key_attr.hash {
        return h(key);
    }
    let mut code: u64 = 0;
    match key_attr.type_ {
        DictType::Char => {
            // C treats char as signed on x86-64 GCC default — sign-extend.
            code = (key[0] as i8) as i64 as u64;
        }
        DictType::WChar => {
            let v = i32::from_le_bytes([key[0], key[1], key[2], key[3]]);
            code = v as i64 as u64;
        }
        DictType::I32 => {
            let v = i32::from_le_bytes([key[0], key[1], key[2], key[3]]);
            code = v as i64 as u64;
        }
        DictType::U32 => {
            let v = u32::from_le_bytes([key[0], key[1], key[2], key[3]]);
            code = v as u64;
        }
        DictType::F32 => {
            let v = f32::from_le_bytes([key[0], key[1], key[2], key[3]]);
            // Match C semantics: implicit float-to-uint64 truncation.
            code = v as u64;
        }
        DictType::I64 => {
            let arr: [u8; 8] = key[..8].try_into().unwrap_or([0u8; 8]);
            let v = i64::from_le_bytes(arr);
            code = v as u64;
        }
        DictType::U64 => {
            let arr: [u8; 8] = key[..8].try_into().unwrap_or([0u8; 8]);
            code = u64::from_le_bytes(arr);
        }
        DictType::F64 => {
            let arr: [u8; 8] = key[..8].try_into().unwrap_or([0u8; 8]);
            let v = f64::from_le_bytes(arr);
            code = v as u64;
        }
        DictType::Ptr => {
            let arr: [u8; 8] = key[..8].try_into().unwrap_or([0u8; 8]);
            code = u64::from_le_bytes(arr);
        }
        DictType::Str => {
            for &b in key {
                code = (code.wrapping_mul(HASH_BASE).wrapping_add(b as u64)) % HASH_MOD;
            }
        }
        DictType::Struct => {
            let len = key_attr.size.min(key.len());
            for &b in &key[..len] {
                code = (code.wrapping_mul(HASH_BASE).wrapping_add(b as u64)) % HASH_MOD;
            }
        }
    }
    code
}

/// Determine if two prepared key buffers represent the same key.
fn keys_match_internal(key_attr: &DictKeyAttr, a: &[u8], b: &[u8]) -> bool {
    if let Some(cmpr) = key_attr.cmpr {
        return cmpr(a, b) == 0;
    }
    // For STR keys, compare full slice contents. For others, both buffers
    // have been normalized to `key_attr.size` bytes by `prepare_key_bytes`.
    a == b
}

// =============================================================================
// Public API
// =============================================================================

/// Create a dictionary using detailed arguments. Matches C's dict_create().
pub fn dict_create(args: DictArgs) -> Dict {
    build_dict(args)
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
        alloc: DictAlloc {
            malloc: None,
            free: None,
        },
    };
    build_dict(args)
}

/// Destroy a dictionary. Matches C's dict_destroy().
/// In Rust, memory is freed automatically, but we emulate calling destructors if provided.
pub fn dict_destroy(dict: &mut Dict) {
    let key_copy = dict.key.copy;
    let key_free = dict.key.free;
    let val_size = dict.val.size;
    let val_free = dict.val.free;
    for bucket in dict.buckets.iter_mut() {
        for elem in bucket.elements.iter_mut() {
            if key_copy.is_some() {
                if let Some(f) = key_free {
                    f(&mut elem.key);
                }
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

/// Retrieve or create a value from the dictionary.
pub fn dict_get<'dict>(dict: &'dict mut Dict, key_data: &[u8]) -> Option<&'dict mut [u8]> {
    let key_bytes = prepare_key_bytes(dict, key_data);
    let code = compute_hash_internal(&dict.key, &key_bytes);
    let idx = (code % dict.mod_ as u64) as usize;

    // Step 1: attempt to find an existing element.
    let existing = {
        let key_attr = &dict.key;
        dict.buckets[idx].elements.iter().position(|e| {
            e.code == code && keys_match_internal(key_attr, &e.key, &key_bytes)
        })
    };

    if let Some(p) = existing {
        return Some(&mut dict.buckets[idx].elements[p].val);
    }

    // Step 2: insert a new element.
    let val_size = dict.val.size;
    dict.buckets[idx].elements.push(DictElem {
        code,
        key: key_bytes.clone(),
        val: vec![0u8; val_size],
    });
    dict.count += 1;
    let bucket_size_after = dict.buckets[idx].elements.len();

    // Step 3: reshape if the bucket grew too large.
    if bucket_size_after > dict.mod_ {
        let _ = dict_reshape(dict, 1);
        let new_idx = (code % dict.mod_ as u64) as usize;
        let key_attr = &dict.key;
        let new_pos = dict.buckets[new_idx]
            .elements
            .iter()
            .position(|e| e.code == code && keys_match_internal(key_attr, &e.key, &key_bytes))
            .expect("element must still exist after reshape");
        return Some(&mut dict.buckets[new_idx].elements[new_pos].val);
    }

    let last = dict.buckets[idx].elements.len() - 1;
    Some(&mut dict.buckets[idx].elements[last].val)
}

/// Remove a value from the dictionary. Returns true if the element was found and removed.
pub fn dict_remove(dict: &mut Dict, key_data: &[u8]) -> bool {
    let key_bytes = prepare_key_bytes(dict, key_data);
    let code = compute_hash_internal(&dict.key, &key_bytes);
    let idx = (code % dict.mod_ as u64) as usize;

    let pos = {
        let key_attr = &dict.key;
        dict.buckets[idx].elements.iter().position(|e| {
            e.code == code && keys_match_internal(key_attr, &e.key, &key_bytes)
        })
    };

    if let Some(p) = pos {
        let mut elem = dict.buckets[idx].elements.remove(p);
        let key_copy = dict.key.copy;
        let key_free = dict.key.free;
        let val_size = dict.val.size;
        let val_free = dict.val.free;
        if key_copy.is_some() {
            if let Some(f) = key_free {
                f(&mut elem.key);
            }
        }
        if val_size != 0 {
            if let Some(f) = val_free {
                f(&mut elem.val);
            }
        }
        if dict.count > 0 {
            dict.count -= 1;
        }
        true
    } else {
        false
    }
}

/// Check if a key exists in the dictionary.
pub fn dict_has(dict: &Dict, key_data: &[u8]) -> bool {
    let key_bytes = prepare_key_bytes(dict, key_data);
    let code = compute_hash_internal(&dict.key, &key_bytes);
    let idx = (code % dict.mod_ as u64) as usize;
    let key_attr = &dict.key;
    dict.buckets[idx]
        .elements
        .iter()
        .any(|e| e.code == code && keys_match_internal(key_attr, &e.key, &key_bytes))
}

/// Return the number of elements in the dictionary.
pub fn dict_len(dict: &Dict) -> usize {
    let mut total = 0usize;
    for bucket in &dict.buckets {
        total += bucket.elements.len();
    }
    total
}

/// Return a snapshot of all keys as a flat byte slice. The buffer is leaked
/// to provide a `'static` lifetime, matching the C version's caller-frees-it
/// model (without resorting to unsafe).
pub fn dict_key(dict: &Dict, size: &mut usize) -> Option<&'static [u8]> {
    let len = dict_len(dict);
    *size = len;
    if len == 0 {
        return None;
    }
    let mut buf: Vec<u8> = Vec::new();
    for bucket in &dict.buckets {
        for elem in &bucket.elements {
            buf.extend_from_slice(&elem.key);
        }
    }
    let leaked: &'static [u8] = Box::leak(buf.into_boxed_slice());
    Some(leaked)
}

/// Serialize a dictionary into a contiguous Vec<u8>.
pub fn dict_serialize(dict: &Dict, bytes: &mut usize) -> Option<Vec<u8>> {
    let count = dict_len(dict) as u32;
    let key_size = dict.key.size as u32;
    let val_size = dict.val.size as u32;

    let mut output: Vec<u8> = Vec::new();
    output.extend_from_slice(&key_size.to_le_bytes());
    output.extend_from_slice(&val_size.to_le_bytes());
    output.extend_from_slice(&count.to_le_bytes());

    if dict.key.type_ == DictType::Str {
        // Per-element record: u32 strlen + val_size bytes; then the string area.
        let mut strings: Vec<u8> = Vec::new();
        for bucket in &dict.buckets {
            for elem in &bucket.elements {
                let strlen = elem.key.len() as u32;
                output.extend_from_slice(&strlen.to_le_bytes());
                if dict.val.size != 0 {
                    let pad = dict.val.size.saturating_sub(elem.val.len());
                    output.extend_from_slice(&elem.val);
                    if pad > 0 {
                        output.extend(std::iter::repeat(0u8).take(pad));
                    }
                }
                strings.extend_from_slice(&elem.key);
            }
        }
        output.extend_from_slice(&strings);
    } else {
        for bucket in &dict.buckets {
            for elem in &bucket.elements {
                // Write key bytes (already exactly key.size in length).
                if elem.key.len() == dict.key.size {
                    output.extend_from_slice(&elem.key);
                } else {
                    // Defensive: pad/truncate.
                    let mut tmp = vec![0u8; dict.key.size];
                    let n = elem.key.len().min(dict.key.size);
                    tmp[..n].copy_from_slice(&elem.key[..n]);
                    output.extend_from_slice(&tmp);
                }
                // Write val bytes.
                if elem.val.len() == dict.val.size {
                    output.extend_from_slice(&elem.val);
                } else {
                    let mut tmp = vec![0u8; dict.val.size];
                    let n = elem.val.len().min(dict.val.size);
                    tmp[..n].copy_from_slice(&elem.val[..n]);
                    output.extend_from_slice(&tmp);
                }
            }
        }
    }

    *bytes = output.len();
    Some(output)
}

/// Deserialize a dictionary from a slice.
pub fn dict_deserialize(args: DictArgs, data: &[u8]) -> Dict {
    let mut dict = build_dict(args);

    if data.len() < 12 {
        return dict;
    }

    let header_key_size = read_u32(&data[0..4]) as usize;
    let header_val_size = read_u32(&data[4..8]) as usize;
    let count = read_u32(&data[8..12]) as usize;

    if header_key_size != dict.key.size || header_val_size != dict.val.size {
        return dict;
    }

    let mut ptr = 12usize;

    if dict.key.type_ == DictType::Str {
        // Compute element record size and string area start.
        let elem_size = 4usize + dict.val.size;
        let str_area_start = 12usize.saturating_add(count.saturating_mul(elem_size));
        let mut str_ptr = str_area_start;

        for _ in 0..count {
            if ptr + 4 > data.len() {
                break;
            }
            let strlen = read_u32(&data[ptr..ptr + 4]) as usize;
            ptr += 4;

            let val_bytes: Vec<u8> = if dict.val.size > 0 && ptr + dict.val.size <= data.len() {
                data[ptr..ptr + dict.val.size].to_vec()
            } else {
                vec![0u8; dict.val.size]
            };
            ptr += dict.val.size;

            let key_bytes: Vec<u8> = if str_ptr + strlen <= data.len() {
                data[str_ptr..str_ptr + strlen].to_vec()
            } else {
                Vec::new()
            };
            str_ptr += strlen;

            insert_raw(&mut dict, key_bytes, val_bytes);
        }
    } else {
        let key_size = dict.key.size;
        let val_size = dict.val.size;
        for _ in 0..count {
            if ptr + key_size > data.len() {
                break;
            }
            let key_bytes = data[ptr..ptr + key_size].to_vec();
            ptr += key_size;

            let val_bytes: Vec<u8> = if val_size > 0 && ptr + val_size <= data.len() {
                data[ptr..ptr + val_size].to_vec()
            } else {
                vec![0u8; val_size]
            };
            ptr += val_size;

            insert_raw(&mut dict, key_bytes, val_bytes);
        }
    }

    // Reshape if any bucket has grown past DEFAULT_MOD.
    let max = dict
        .buckets
        .iter()
        .map(|b| b.elements.len())
        .max()
        .unwrap_or(0);
    if max > DEFAULT_MOD {
        let step = max / DEFAULT_MOD;
        let _ = dict_reshape(&mut dict, step);
    }

    dict
}

/// Internal helper: insert a fully constructed element bypassing duplicate
/// detection (used by deserialize where uniqueness is guaranteed).
fn insert_raw(dict: &mut Dict, key: Vec<u8>, val: Vec<u8>) {
    let code = compute_hash_internal(&dict.key, &key);
    let idx = (code % dict.mod_ as u64) as usize;
    dict.buckets[idx].elements.push(DictElem { code, key, val });
    dict.count += 1;
}

/// Convenience function to create a dictionary using inline arguments, mirroring the
/// C macro dict_create_args(...).
pub fn dict_create_args(args: DictArgs) -> Dict {
    build_dict(args)
}

/// The original dict_key_equals.
pub fn dict_key_equals(dict: &Dict, a: &[u8], b: &[u8]) -> bool {
    keys_match_internal(&dict.key, a, b)
}

/// Not used in this design, but signature is kept.
pub fn dict_delete_node(_list: &mut DictBucket, _curr: &mut DictElem) {
    // no-op in this safe design
}

/// The original dict_free_val.
pub fn dict_free_val(dict: &Dict, val: &mut [u8]) {
    if let Some(f) = dict.val.free {
        f(val);
    }
}

/// Not used in pure Rust version, matching signature only.
pub fn dict_get_key(_dict: &Dict) -> Option<&mut [u8]> {
    None
}

/// Internal function to reshape the dictionary.
pub fn dict_reshape(dict: &mut Dict, step: usize) -> bool {
    let old_size = dict.mod_;
    let new_size = old_size
        .checked_mul(step.max(1))
        .and_then(|v| v.checked_mul(DEFAULT_STEP))
        .unwrap_or(old_size * 2);

    if new_size == 0 {
        return false;
    }

    let mut new_buckets: Vec<DictBucket> = (0..new_size)
        .map(|_| DictBucket {
            elements: Vec::new(),
        })
        .collect();

    let old_buckets = std::mem::take(&mut dict.buckets);
    for bucket in old_buckets {
        for elem in bucket.elements {
            let new_idx = (elem.code % new_size as u64) as usize;
            new_buckets[new_idx].elements.push(elem);
        }
    }

    dict.buckets = new_buckets;
    dict.mod_ = new_size;
    true
}

/// Internal function to free a node.
pub fn dict_free_node(_dict: &Dict, _node: &mut DictElem) {
    // No-op: memory is managed by Rust's ownership system.
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
    compute_hash_internal(&dict.key, key)
}
