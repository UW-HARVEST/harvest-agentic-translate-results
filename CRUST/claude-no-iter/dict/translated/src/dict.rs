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

// ----- Internal helpers -----

#[inline]
fn align_up_uintptr(size: usize) -> usize {
    let a = std::mem::size_of::<usize>();
    (size + (a - 1)) & !(a - 1)
}

#[inline]
fn read_u32_le(b: &[u8]) -> u32 {
    let mut buf = [0u8; 4];
    let n = b.len().min(4);
    buf[..n].copy_from_slice(&b[..n]);
    u32::from_ne_bytes(buf)
}

#[inline]
fn read_i32_le(b: &[u8]) -> i32 {
    let mut buf = [0u8; 4];
    let n = b.len().min(4);
    buf[..n].copy_from_slice(&b[..n]);
    i32::from_ne_bytes(buf)
}

#[inline]
fn read_u64_le(b: &[u8]) -> u64 {
    let mut buf = [0u8; 8];
    let n = b.len().min(8);
    buf[..n].copy_from_slice(&b[..n]);
    u64::from_ne_bytes(buf)
}

#[inline]
fn read_i64_le(b: &[u8]) -> i64 {
    let mut buf = [0u8; 8];
    let n = b.len().min(8);
    buf[..n].copy_from_slice(&b[..n]);
    i64::from_ne_bytes(buf)
}

#[inline]
fn read_f32_le(b: &[u8]) -> f32 {
    let mut buf = [0u8; 4];
    let n = b.len().min(4);
    buf[..n].copy_from_slice(&b[..n]);
    f32::from_ne_bytes(buf)
}

#[inline]
fn read_f64_le(b: &[u8]) -> f64 {
    let mut buf = [0u8; 8];
    let n = b.len().min(8);
    buf[..n].copy_from_slice(&b[..n]);
    f64::from_ne_bytes(buf)
}

/// Compute the hash code for a key, matching the C `dict_get_hash` semantics.
fn compute_hash(dict: &Dict, key_data: &[u8]) -> u64 {
    if let Some(hash_fn) = dict.key.hash {
        return hash_fn(key_data);
    }
    match dict.key.type_ {
        DictType::Char => {
            if key_data.is_empty() {
                0
            } else {
                // C: code = *(char*) key  (signed char -> uint64 with sign extension)
                key_data[0] as i8 as i64 as u64
            }
        }
        DictType::WChar => read_u32_le(key_data) as u64,
        DictType::I32 => read_i32_le(key_data) as i64 as u64,
        DictType::U32 => read_u32_le(key_data) as u64,
        DictType::F32 => {
            // C: code = *(float*) key  -- conversion truncates toward zero
            let v = read_f32_le(key_data);
            v as u64
        }
        DictType::I64 => read_i64_le(key_data) as u64,
        DictType::U64 => read_u64_le(key_data),
        DictType::F64 => {
            let v = read_f64_le(key_data);
            v as u64
        }
        DictType::Ptr => read_u64_le(key_data),
        DictType::Str => {
            // Hash all bytes provided.
            let mut code: u64 = 0;
            for &b in key_data {
                code = (code.wrapping_mul(HASH_BASE) + b as u64) % HASH_MOD;
            }
            code
        }
        DictType::Struct => {
            // C uses dict.key.size bytes; we use whatever bytes are provided
            // (so callers can pass smaller keys safely).
            let length = key_data.len();
            let mut code: u64 = 0;
            for i in 0..length {
                code = (code.wrapping_mul(HASH_BASE) + key_data[i] as u64) % HASH_MOD;
            }
            code
        }
    }
}

/// Compare two keys for equality, matching the C `dict_get` comparison logic.
fn keys_equal(dict: &Dict, stored: &[u8], input: &[u8]) -> bool {
    if let Some(cmpr) = dict.key.cmpr {
        return cmpr(stored, input) == 0;
    }
    // For both Str and other types: compare byte slices directly.
    stored == input
}

/// Create a dictionary using detailed arguments. Matches C's dict_create().
pub fn dict_create(args: DictArgs) -> Dict {
    let key_size: usize = match args.key.type_ {
        DictType::Char => 1,
        DictType::WChar => 4, // wchar_t on most Linux systems
        DictType::I32 => 4,
        DictType::U32 => 4,
        DictType::F32 => 4,
        DictType::I64 => 8,
        DictType::U64 => 8,
        DictType::F64 => 8,
        DictType::Ptr => std::mem::size_of::<usize>(),
        DictType::Str => std::mem::size_of::<usize>(),
        DictType::Struct => align_up_uintptr(args.key.size),
    };
    let val_size = align_up_uintptr(args.val.size);

    let mut key = args.key.clone();
    key.size = key_size;
    let mut val = args.val.clone();
    val.size = val_size;

    let buckets: Vec<DictBucket> = (0..DEFAULT_MOD)
        .map(|_| DictBucket { elements: Vec::new() })
        .collect();

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
    // Run optional destructors on each key/value, then drop the storage.
    for bucket in dict.buckets.iter_mut() {
        for elem in bucket.elements.iter_mut() {
            // Call key destructor if a custom copy/free was provided.
            if dict.key.copy.is_some() {
                if let Some(free_fn) = dict.key.free {
                    free_fn(&mut elem.key);
                }
            }
            if let Some(free_fn) = dict.val.free {
                if !elem.val.is_empty() {
                    free_fn(&mut elem.val);
                }
            }
        }
        bucket.elements.clear();
    }
    dict.buckets.clear();
    dict.key_temp.clear();
    dict.keys_dump.clear();
    dict.count = 0;
    dict.mod_ = 0;
}
/// Retrieve or create a value from the dictionary. In C, this used varargs. Here, we
/// accept a slice of bytes as the key. Returns a mutable slice of the value,
/// or None if something went wrong. Matches C's dict_get(dict_t*, ...).
pub fn dict_get<'dict>(dict: &'dict mut Dict, key_data: &[u8]) -> Option<&'dict mut [u8]> {
    let code = compute_hash(dict, key_data);
    let index = (code % dict.mod_ as u64) as usize;

    // 1) Search for an existing matching element.
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

    if let Some(_) = found_idx {
        // No reshape happens; index is still valid.
        let bucket = &mut dict.buckets[index];
        for elem in bucket.elements.iter_mut() {
            if elem.code == code {
                // We can't call `keys_equal` (which borrows dict immutably) here,
                // so we re-check via a local equivalent.
                let matches = match dict.key.cmpr {
                    Some(cmpr) => cmpr(&elem.key, key_data) == 0,
                    None => elem.key.as_slice() == key_data,
                };
                if matches {
                    return Some(elem.val.as_mut_slice());
                }
            }
        }
        return None;
    }

    // 2) Not found: insert a new element.
    let val_size = dict.val.size;
    let new_elem = DictElem {
        code,
        key: key_data.to_vec(),
        val: vec![0u8; val_size],
    };
    dict.buckets[index].elements.push(new_elem);
    dict.count += 1;

    // 3) Possibly reshape if the bucket got too big.
    let bucket_size = dict.buckets[index].elements.len();
    let mut final_index = index;
    if bucket_size > dict.mod_ {
        if !dict_reshape(dict, 1) {
            return None;
        }
        final_index = (code % dict.mod_ as u64) as usize;
    }

    // 4) Locate the (possibly relocated) element and return mutable val slice.
    let bucket = &mut dict.buckets[final_index];
    for elem in bucket.elements.iter_mut() {
        if elem.code == code {
            let matches = match dict.key.cmpr {
                Some(cmpr) => cmpr(&elem.key, key_data) == 0,
                None => elem.key.as_slice() == key_data,
            };
            if matches {
                return Some(elem.val.as_mut_slice());
            }
        }
    }
    None
}
/// Remove a value from the dictionary. Matches C's dict_remove(dict_t*, ...).
/// Returns true if the element was found and removed.
pub fn dict_remove(dict: &mut Dict, key_data: &[u8]) -> bool {
    let code = compute_hash(dict, key_data);
    let index = (code % dict.mod_ as u64) as usize;

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
        // Run user-defined destructors on key/val if any.
        let mut elem = dict.buckets[index].elements.remove(i);
        if dict.key.copy.is_some() {
            if let Some(free_fn) = dict.key.free {
                free_fn(&mut elem.key);
            }
        }
        if let Some(free_fn) = dict.val.free {
            if !elem.val.is_empty() {
                free_fn(&mut elem.val);
            }
        }
        dict.count -= 1;
        return true;
    }
    false
}
/// Check if a key exists in the dictionary. Matches C's dict_has(const dict_t*, ...).
pub fn dict_has(dict: &Dict, key_data: &[u8]) -> bool {
    if dict.mod_ == 0 {
        return false;
    }
    let code = compute_hash(dict, key_data);
    let index = (code % dict.mod_ as u64) as usize;
    let bucket = &dict.buckets[index];
    for elem in bucket.elements.iter() {
        if elem.code == code && keys_equal(dict, &elem.key, key_data) {
            return true;
        }
    }
    false
}
/// Return the number of elements in the dictionary. Matches C's dict_len().
pub fn dict_len(dict: &Dict) -> usize {
    let mut total = 0usize;
    for bucket in dict.buckets.iter() {
        total += bucket.elements.len();
    }
    total
}
/// Return a snapshot of all keys. In safe Rust, we leak a fresh allocation
/// to provide a `'static` slice, mirroring how the C version returns a
/// freshly malloc'd buffer (the caller would `free()` it in C).
pub fn dict_key(dict: &Dict, size: &mut usize) -> Option<&'static [u8]> {
    let count = dict_len(dict);
    *size = count;
    if count == 0 {
        return None;
    }

    let key_size = dict.key.size;
    let mut buf: Vec<u8> = Vec::with_capacity(count * key_size.max(1));
    for bucket in dict.buckets.iter() {
        for elem in bucket.elements.iter() {
            // Pad/truncate each stored key to dict.key.size for a uniform layout.
            if elem.key.len() >= key_size {
                buf.extend_from_slice(&elem.key[..key_size]);
            } else {
                buf.extend_from_slice(&elem.key);
                for _ in elem.key.len()..key_size {
                    buf.push(0);
                }
            }
        }
    }

    let leaked: &'static [u8] = Box::leak(buf.into_boxed_slice());
    Some(leaked)
}
/// Serialize a dictionary into a contiguous Vec<u8>. Matches C's dict_serialize().
pub fn dict_serialize(dict: &Dict, bytes: &mut usize) -> Option<Vec<u8>> {
    let count = dict_len(dict) as u32;
    let key_size_u32 = dict.key.size as u32;
    let val_size_u32 = dict.val.size as u32;

    let mut data: Vec<u8> = Vec::new();
    data.extend_from_slice(&key_size_u32.to_ne_bytes());
    data.extend_from_slice(&val_size_u32.to_ne_bytes());
    data.extend_from_slice(&count.to_ne_bytes());

    let val_size = dict.val.size;

    if dict.key.type_ == DictType::Str {
        // Each entry in the table: 4 bytes strlen + val_size bytes.
        // After the table, all string bytes are concatenated.
        let mut all_strs: Vec<u8> = Vec::new();
        for bucket in dict.buckets.iter() {
            for elem in bucket.elements.iter() {
                let strlen = elem.key.len() as u32;
                data.extend_from_slice(&strlen.to_ne_bytes());
                let mut v = elem.val.clone();
                v.resize(val_size, 0);
                data.extend_from_slice(&v);
                all_strs.extend_from_slice(&elem.key);
            }
        }
        data.extend_from_slice(&all_strs);
    } else {
        let key_size = dict.key.size;
        for bucket in dict.buckets.iter() {
            for elem in bucket.elements.iter() {
                let mut k = elem.key.clone();
                k.resize(key_size, 0);
                data.extend_from_slice(&k);
                let mut v = elem.val.clone();
                v.resize(val_size, 0);
                data.extend_from_slice(&v);
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

    let header_key_size = read_u32_le(&data[0..4]) as usize;
    let header_val_size = read_u32_le(&data[4..8]) as usize;
    let count = read_u32_le(&data[8..12]) as usize;

    if header_key_size != dict.key.size || header_val_size != dict.val.size {
        // Corrupt / mismatched header; return the empty dict.
        return dict;
    }

    let val_size = dict.val.size;
    let key_size = dict.key.size;

    let mut ptr = 12usize;

    if dict.key.type_ == DictType::Str {
        let entry_size = 4 + val_size;
        let table_end = ptr + count * entry_size;
        if table_end > data.len() {
            return dict;
        }
        let mut str_ptr = table_end;
        for _ in 0..count {
            if ptr + 4 + val_size > data.len() {
                return dict;
            }
            let strlen = read_u32_le(&data[ptr..ptr + 4]) as usize;
            ptr += 4;
            let val = data[ptr..ptr + val_size].to_vec();
            ptr += val_size;
            if str_ptr + strlen > data.len() {
                return dict;
            }
            let key = data[str_ptr..str_ptr + strlen].to_vec();
            str_ptr += strlen;

            let code = compute_hash(&dict, &key);
            let index = (code % dict.mod_ as u64) as usize;
            dict.buckets[index].elements.push(DictElem { code, key, val });
            dict.count += 1;
        }
    } else {
        let entry_size = key_size + val_size;
        if ptr + count * entry_size > data.len() {
            return dict;
        }
        for _ in 0..count {
            let key = data[ptr..ptr + key_size].to_vec();
            ptr += key_size;
            let val = data[ptr..ptr + val_size].to_vec();
            ptr += val_size;

            let code = compute_hash(&dict, &key);
            let index = (code % dict.mod_ as u64) as usize;
            dict.buckets[index].elements.push(DictElem { code, key, val });
            dict.count += 1;
        }
    }

    // Possibly reshape based on max bucket size, matching the C reference.
    let mut max = 0usize;
    for bucket in dict.buckets.iter() {
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
    if let Some(free_fn) = dict.val.free {
        free_fn(val);
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
    let mut new_size = old_size.saturating_mul(step).saturating_mul(DEFAULT_STEP);
    if new_size == 0 {
        new_size = DEFAULT_MOD;
    }

    let mut new_buckets: Vec<DictBucket> = (0..new_size)
        .map(|_| DictBucket { elements: Vec::new() })
        .collect();

    let old_buckets = std::mem::take(&mut dict.buckets);
    for old_bucket in old_buckets.into_iter() {
        for elem in old_bucket.elements.into_iter() {
            let index = (elem.code % new_size as u64) as usize;
            new_buckets[index].elements.push(elem);
        }
    }

    dict.mod_ = new_size;
    dict.buckets = new_buckets;
    true
}
/// Internal function to free a node. Matches C's dict_free_node().
pub fn dict_free_node(_dict: &Dict, _node: &mut DictElem) {
    // Memory is managed automatically in safe Rust; nothing to free explicitly.
}
/// Internal function to free a dictionary key. Kept for signature consistency.
pub fn dict_free_key(dict: &Dict, key: &mut [u8]) {
    if dict.key.copy.is_some() {
        if let Some(free_fn) = dict.key.free {
            free_fn(key);
        }
    }
}
/// The original dict_get_hash. Kept for signature consistency but not used internally
/// to avoid borrow conflicts.
pub fn dict_get_hash(dict: &Dict, key: &[u8]) -> u64 {
    compute_hash(dict, key)
}
