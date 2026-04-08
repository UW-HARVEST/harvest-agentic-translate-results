
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

/// Helper: compute the aligned key size for a given type, matching C's dict_create logic.
fn compute_key_size(type_: DictType, user_size: usize) -> usize {
    let ptr_size = std::mem::size_of::<usize>();
    match type_ {
        DictType::Char => std::mem::size_of::<u8>(),
        DictType::WChar => std::mem::size_of::<i32>(), // wchar_t is typically 4 bytes on Linux
        DictType::I32 => 4,
        DictType::U32 => 4,
        DictType::F32 => 4,
        DictType::I64 => 8,
        DictType::U64 => 8,
        DictType::F64 => 8,
        DictType::Ptr => ptr_size,
        DictType::Str => ptr_size, // sizeof(char*) in C
        DictType::Struct => (user_size + (ptr_size - 1)) & !(ptr_size - 1),
    }
}

/// Helper: compute aligned val size
fn compute_val_size(user_size: usize) -> usize {
    let ptr_size = std::mem::size_of::<usize>();
    (user_size + (ptr_size - 1)) & !(ptr_size - 1)
}

/// Helper: compute hash for a key, matching C's dict_get_hash
fn compute_hash(key_attr: &DictKeyAttr, key: &[u8]) -> u64 {
    if let Some(hash_fn) = key_attr.hash {
        return hash_fn(key);
    }
    match key_attr.type_ {
        DictType::Str => {
            // key contains a pointer-sized representation; we treat the key bytes
            // as the actual string bytes for hashing. But in our Rust model,
            // for Str type the key bytes ARE the string bytes directly.
            // Actually, looking at the C code: for DICT_STR, the key field stores
            // a char* (pointer). In our Rust version, key bytes store the actual string.
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
        _ => {
            // For primitive types, interpret the bytes as a u64 value
            // C code casts the value directly to u64
            match key.len() {
                1 => key[0] as u64,
                2 => {
                    let mut buf = [0u8; 2];
                    buf.copy_from_slice(key);
                    u16::from_ne_bytes(buf) as u64
                }
                4 => {
                    let mut buf = [0u8; 4];
                    buf.copy_from_slice(key);
                    u32::from_ne_bytes(buf) as u64
                }
                8 => {
                    let mut buf = [0u8; 8];
                    buf.copy_from_slice(key);
                    u64::from_ne_bytes(buf)
                }
                _ => {
                    // fallback: treat as struct-like
                    let mut code: u64 = 0;
                    for &b in key.iter() {
                        code = (code.wrapping_mul(HASH_BASE).wrapping_add(b as u64)) % HASH_MOD;
                    }
                    code
                }
            }
        }
    }
}

/// Helper: compare two keys for equality
fn keys_equal(key_attr: &DictKeyAttr, a: &[u8], b: &[u8]) -> bool {
    if let Some(cmpr) = key_attr.cmpr {
        return cmpr(a, b) == 0;
    }
    match key_attr.type_ {
        DictType::Str => a == b, // In our model, Str keys store actual string bytes
        _ => a == b,
    }
}

/// Create a dictionary using detailed arguments. Matches C's dict_create().
pub fn dict_create(args: DictArgs) -> Dict {
    let key_size = compute_key_size(args.key.type_, args.key.size);
    let val_size = compute_val_size(args.val.size);

    Dict {
        key: DictKeyAttr {
            size: key_size,
            ..args.key
        },
        val: DictValAttr {
            size: val_size,
            ..args.val
        },
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
    for bucket in dict.buckets.iter_mut() {
        for elem in bucket.elements.iter_mut() {
            if let Some(key_free) = dict.key.free {
                key_free(&mut elem.key);
            }
            if dict.val.size != 0 {
                if let Some(val_free) = dict.val.free {
                    val_free(&mut elem.val);
                }
            }
        }
        bucket.elements.clear();
    }
    dict.buckets.clear();
    dict.key_temp.clear();
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
    // Prepare key
    let key_bytes = prepare_key(&dict.key, key_data);
    let code = compute_hash(&dict.key, &key_bytes);
    let index = (code as usize) % dict.mod_;

    // Check if key already exists
    let found = dict.buckets[index].elements.iter().position(|elem| {
        elem.code == code && keys_equal(&dict.key, &elem.key, &key_bytes)
    });

    if let Some(pos) = found {
        return Some(&mut dict.buckets[index].elements[pos].val);
    }

    // Insert new element
    let new_elem = DictElem {
        code,
        key: key_bytes,
        val: vec![0u8; dict.val.size],
    };
    dict.buckets[index].elements.push(new_elem);
    dict.count += 1;

    let bucket_size = dict.buckets[index].elements.len();
    if bucket_size > dict.mod_ {
        dict_reshape(dict, 1);
        // After reshape, need to find the element again
        let new_index = (code as usize) % dict.mod_;
        let pos = dict.buckets[new_index].elements.iter().position(|elem| {
            elem.code == code
        });
        if let Some(p) = pos {
            return Some(&mut dict.buckets[new_index].elements[p].val);
        }
        return None;
    }

    let last = dict.buckets[index].elements.len() - 1;
    Some(&mut dict.buckets[index].elements[last].val)
}

/// Helper to prepare key bytes from input data
fn prepare_key(key_attr: &DictKeyAttr, key_data: &[u8]) -> Vec<u8> {
    if key_attr.copy.is_some() {
        let mut dest = vec![0u8; key_attr.size];
        (key_attr.copy.unwrap())(&mut dest, key_data);
        dest
    } else {
        match key_attr.type_ {
            DictType::Str => {
                // For Str type, key_data is the raw string bytes
                key_data.to_vec()
            }
            DictType::Struct => {
                // Copy up to key.size bytes
                let mut buf = vec![0u8; key_attr.size];
                let copy_len = key_data.len().min(key_attr.size);
                buf[..copy_len].copy_from_slice(&key_data[..copy_len]);
                buf
            }
            _ => {
                // Primitive types: copy the bytes as-is
                key_data.to_vec()
            }
        }
    }
}

/// Remove a value from the dictionary. Matches C's dict_remove(dict_t*, ...).
/// Returns true if the element was found and removed.
pub fn dict_remove(dict: &mut Dict, key_data: &[u8]) -> bool {
    let key_bytes = prepare_key(&dict.key, key_data);
    let code = compute_hash(&dict.key, &key_bytes);
    let index = (code as usize) % dict.mod_;

    let pos = dict.buckets[index].elements.iter().position(|elem| {
        elem.code == code && keys_equal(&dict.key, &elem.key, &key_bytes)
    });

    if let Some(p) = pos {
        let mut removed = dict.buckets[index].elements.remove(p);
        if let Some(key_free) = dict.key.free {
            key_free(&mut removed.key);
        }
        if let Some(val_free) = dict.val.free {
            val_free(&mut removed.val);
        }
        dict.count -= 1;
        true
    } else {
        false
    }
}
/// Check if a key exists in the dictionary. Matches C's dict_has(const dict_t*, ...).
pub fn dict_has(dict: &Dict, key_data: &[u8]) -> bool {
    let key_bytes = prepare_key(&dict.key, key_data);
    let code = compute_hash(&dict.key, &key_bytes);
    let index = (code as usize) % dict.mod_;

    dict.buckets[index].elements.iter().any(|elem| {
        elem.code == code && keys_equal(&dict.key, &elem.key, &key_bytes)
    })
}
/// Return the number of elements in the dictionary. Matches C's dict_len().
pub fn dict_len(dict: &Dict) -> usize {
    dict.buckets.iter().map(|b| b.elements.len()).sum()
}
/// Return a snapshot of all keys. In C, it returns a newly allocated array of all keys
/// (size = key.size * dict_len). This is not thread-safe in the original C usage. In
/// safe Rust, we simulate returning a static buffer by leaking the allocation. This
/// avoids unsafe code, but does leak memory for each call. Matches C's dict_key().
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
            let offset = key_size * idx;
            let copy_len = elem.key.len().min(key_size);
            arr[offset..offset + copy_len].copy_from_slice(&elem.key[..copy_len]);
            idx += 1;
            if idx >= *size {
                let leaked = arr.leak();
                return Some(leaked);
            }
        }
    }

    let leaked = arr.leak();
    Some(leaked)
}
/// Serialize a dictionary into a contiguous Vec<u8>. Matches C's dict_serialize().
pub fn dict_serialize(dict: &Dict, bytes: &mut usize) -> Option<Vec<u8>> {
    let size = dict_len(dict) as u32;
    let key_size = dict.key.size as u32;
    let val_size = dict.val.size as u32;

    let elem_size = if dict.key.type_ == DictType::Str {
        4 + dict.val.size // sizeof(uint32_t) + val_size
    } else {
        dict.key.size + dict.val.size
    };

    // Calculate total size
    let mut total = 12 + (size as usize) * elem_size; // 3 * sizeof(u32) header

    // For Str type, also need space for the actual string data
    let mut strlen_table: Vec<u32> = Vec::new();
    if dict.key.type_ == DictType::Str {
        for bucket in &dict.buckets {
            for elem in &bucket.elements {
                let slen = elem.key.len() as u32;
                strlen_table.push(slen);
                total += slen as usize;
            }
        }
    }

    *bytes = total;

    let mut data = vec![0u8; total];
    let mut ptr = 0;

    // Write header: key_size, val_size, count
    data[ptr..ptr + 4].copy_from_slice(&key_size.to_ne_bytes());
    ptr += 4;
    data[ptr..ptr + 4].copy_from_slice(&val_size.to_ne_bytes());
    ptr += 4;
    data[ptr..ptr + 4].copy_from_slice(&size.to_ne_bytes());
    ptr += 4;

    if dict.key.type_ == DictType::Str {
        let mut str_idx = 0;
        let mut str_ptr = ptr + (size as usize) * elem_size;
        for bucket in &dict.buckets {
            for elem in &bucket.elements {
                let slen = strlen_table[str_idx];
                // Write string length
                data[ptr..ptr + 4].copy_from_slice(&slen.to_ne_bytes());
                ptr += 4;
                // Write value
                let val_sz = dict.val.size;
                let copy_len = elem.val.len().min(val_sz);
                data[ptr..ptr + copy_len].copy_from_slice(&elem.val[..copy_len]);
                ptr += val_sz;
                // Write string data at str_ptr
                data[str_ptr..str_ptr + slen as usize].copy_from_slice(&elem.key[..slen as usize]);
                str_ptr += slen as usize;
                str_idx += 1;
            }
        }
    } else {
        for bucket in &dict.buckets {
            for elem in &bucket.elements {
                // Write key
                let key_sz = dict.key.size;
                let copy_len = elem.key.len().min(key_sz);
                data[ptr..ptr + copy_len].copy_from_slice(&elem.key[..copy_len]);
                ptr += key_sz;
                // Write value
                let val_sz = dict.val.size;
                let copy_len = elem.val.len().min(val_sz);
                data[ptr..ptr + copy_len].copy_from_slice(&elem.val[..copy_len]);
                ptr += val_sz;
            }
        }
    }

    Some(data)
}
/// Deserialize a dictionary from a slice. Matches C's dict_deserialize().
pub fn dict_deserialize(args: DictArgs, data: &[u8]) -> Dict {
    let mut ptr = 0;

    // Read header
    let mut buf4 = [0u8; 4];
    buf4.copy_from_slice(&data[ptr..ptr + 4]);
    let _stored_key_size = u32::from_ne_bytes(buf4) as usize;
    ptr += 4;
    buf4.copy_from_slice(&data[ptr..ptr + 4]);
    let _stored_val_size = u32::from_ne_bytes(buf4) as usize;
    ptr += 4;
    buf4.copy_from_slice(&data[ptr..ptr + 4]);
    let count = u32::from_ne_bytes(buf4) as usize;
    ptr += 4;

    let key_size = compute_key_size(args.key.type_, args.key.size);
    let val_size = compute_val_size(args.val.size);

    // Create the dict
    let mut dict = Dict {
        key: DictKeyAttr {
            size: key_size,
            ..args.key
        },
        val: DictValAttr {
            size: val_size,
            ..args.val
        },
        alloc: args.alloc,
        mod_: DEFAULT_MOD,
        buckets: (0..DEFAULT_MOD).map(|_| DictBucket { elements: Vec::new() }).collect(),
        key_temp: vec![0u8; key_size],
        keys_dump: Vec::new(),
        count: 0,
    };

    let elem_size = if dict.key.type_ == DictType::Str {
        4 + dict.val.size
    } else {
        dict.key.size + dict.val.size
    };

    if dict.key.type_ == DictType::Str {
        let mut str_ptr = ptr + count * elem_size;
        for _ in 0..count {
            buf4.copy_from_slice(&data[ptr..ptr + 4]);
            let slen = u32::from_ne_bytes(buf4) as usize;
            ptr += 4;

            let mut val = vec![0u8; dict.val.size];
            let copy_len = dict.val.size;
            val[..copy_len].copy_from_slice(&data[ptr..ptr + copy_len]);
            ptr += dict.val.size;

            let key_bytes = data[str_ptr..str_ptr + slen].to_vec();
            str_ptr += slen;

            let code = compute_hash(&dict.key, &key_bytes);
            let index = (code as usize) % dict.mod_;
            dict.buckets[index].elements.push(DictElem {
                code,
                key: key_bytes,
                val,
            });
            dict.count += 1;
        }
    } else {
        for _ in 0..count {
            let mut key_bytes = vec![0u8; dict.key.size];
            key_bytes.copy_from_slice(&data[ptr..ptr + dict.key.size]);
            ptr += dict.key.size;

            let mut val = vec![0u8; dict.val.size];
            val.copy_from_slice(&data[ptr..ptr + dict.val.size]);
            ptr += dict.val.size;

            let code = compute_hash(&dict.key, &key_bytes);
            let index = (code as usize) % dict.mod_;
            dict.buckets[index].elements.push(DictElem {
                code,
                key: key_bytes,
                val,
            });
            dict.count += 1;
        }
    }

    // Reshape if needed (matching C logic)
    let mut max_bucket = 0;
    for bucket in &dict.buckets {
        if bucket.elements.len() > max_bucket {
            max_bucket = bucket.elements.len();
        }
    }
    if max_bucket > DEFAULT_MOD {
        let step = max_bucket / DEFAULT_MOD;
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
    let new_size = dict.mod_ * step * DEFAULT_STEP;
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
    // no-op in safe Rust - memory managed automatically
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
    compute_hash(&dict.key, key)
}
