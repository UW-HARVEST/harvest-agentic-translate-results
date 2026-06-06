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

// ---------- Internal helpers ----------

fn align_size(size: usize) -> usize {
    let align = std::mem::size_of::<usize>();
    (size + align - 1) & !(align - 1)
}

fn natural_key_size(key_type: DictType, struct_size: usize) -> usize {
    match key_type {
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
        DictType::Struct => align_size(struct_size),
    }
}

fn read_u32_le(data: &[u8]) -> u32 {
    let mut arr = [0u8; 4];
    let n = data.len().min(4);
    arr[..n].copy_from_slice(&data[..n]);
    u32::from_ne_bytes(arr)
}

fn read_i32_le(data: &[u8]) -> i32 {
    read_u32_le(data) as i32
}

fn read_u64_le(data: &[u8]) -> u64 {
    let mut arr = [0u8; 8];
    let n = data.len().min(8);
    arr[..n].copy_from_slice(&data[..n]);
    u64::from_ne_bytes(arr)
}

fn read_f32_le(data: &[u8]) -> f32 {
    let mut arr = [0u8; 4];
    let n = data.len().min(4);
    arr[..n].copy_from_slice(&data[..n]);
    f32::from_ne_bytes(arr)
}

fn read_f64_le(data: &[u8]) -> f64 {
    let mut arr = [0u8; 8];
    let n = data.len().min(8);
    arr[..n].copy_from_slice(&data[..n]);
    f64::from_ne_bytes(arr)
}

fn compute_hash_for(
    key_type: DictType,
    hash_fn: Option<DictHash>,
    key_bytes: &[u8],
) -> u64 {
    if let Some(h) = hash_fn {
        return h(key_bytes);
    }
    match key_type {
        DictType::Char => {
            if key_bytes.is_empty() {
                0
            } else {
                // signed char promoted to int then to uint64 (sign-extended)
                (key_bytes[0] as i8) as i64 as u64
            }
        }
        DictType::WChar | DictType::I32 => {
            // sign-extended to u64
            read_i32_le(key_bytes) as i64 as u64
        }
        DictType::U32 => read_u32_le(key_bytes) as u64,
        DictType::F32 => {
            // C casts float to uint64_t (truncates float value to integer)
            let f = read_f32_le(key_bytes);
            if f.is_finite() && f >= 0.0 && f < (u64::MAX as f32) {
                f as u64
            } else if f.is_finite() && f < 0.0 {
                // Negative: undefined-ish; emulate "as u64" which clamps to 0
                0
            } else {
                0
            }
        }
        DictType::I64 => read_u64_le(key_bytes), // bit pattern: i64 -> u64
        DictType::U64 => read_u64_le(key_bytes),
        DictType::F64 => {
            let f = read_f64_le(key_bytes);
            if f.is_finite() && f >= 0.0 && f < (u64::MAX as f64) {
                f as u64
            } else if f.is_finite() && f < 0.0 {
                0
            } else {
                0
            }
        }
        DictType::Ptr => read_u64_le(key_bytes),
        DictType::Str | DictType::Struct => {
            let mut code: u64 = 0;
            for &b in key_bytes {
                code = (code.wrapping_mul(HASH_BASE).wrapping_add(b as u64)) % HASH_MOD;
            }
            code
        }
    }
}

fn keys_match(
    cmpr: Option<DictCmpr>,
    stored: &[u8],
    incoming: &[u8],
) -> bool {
    if let Some(c) = cmpr {
        c(stored, incoming) == 0
    } else {
        stored == incoming
    }
}

/// Create a dictionary using detailed arguments. Matches C's dict_create().
pub fn dict_create(args: DictArgs) -> Dict {
    let key_size = natural_key_size(args.key.type_, args.key.size);
    let val_size = align_size(args.val.size);

    let mut key = args.key;
    key.size = key_size;
    let mut val = args.val;
    val.size = val_size;

    let mod_ = DEFAULT_MOD;
    let buckets: Vec<DictBucket> = (0..mod_)
        .map(|_| DictBucket { elements: Vec::new() })
        .collect();

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
                if dict.key.copy.is_some() {
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
    let code = compute_hash_for(dict.key.type_, dict.key.hash, key_data);
    let cmpr = dict.key.cmpr;

    let index = (code % dict.mod_ as u64) as usize;

    // Step 1: search for an existing element at `index`.
    let mut existing_idx: Option<usize> = None;
    {
        let bucket = &dict.buckets[index];
        for (i, elem) in bucket.elements.iter().enumerate() {
            if elem.code != code {
                continue;
            }
            if keys_match(cmpr, &elem.key, key_data) {
                existing_idx = Some(i);
                break;
            }
        }
    }

    if existing_idx.is_none() {
        // Step 2: insert a new element with zero-filled value.
        let new_elem = DictElem {
            code,
            key: key_data.to_vec(),
            val: vec![0u8; dict.val.size],
        };
        dict.buckets[index].elements.push(new_elem);
        dict.count += 1;

        // Step 3: possibly reshape.
        let bucket_size = dict.buckets[index].elements.len();
        if bucket_size > dict.mod_ {
            if !dict_reshape(dict, 1) {
                return None;
            }
        }
    }

    // Step 4: re-find the element (index might have changed after reshape).
    let new_index = (code % dict.mod_ as u64) as usize;
    let bucket = &mut dict.buckets[new_index];
    for elem in bucket.elements.iter_mut() {
        if elem.code != code {
            continue;
        }
        if keys_match(cmpr, &elem.key, key_data) {
            return Some(elem.val.as_mut_slice());
        }
    }
    None
}
/// Remove a value from the dictionary. Matches C's dict_remove(dict_t*, ...).
/// Returns true if the element was found and removed.
pub fn dict_remove(dict: &mut Dict, key_data: &[u8]) -> bool {
    let code = compute_hash_for(dict.key.type_, dict.key.hash, key_data);
    let cmpr = dict.key.cmpr;
    let index = (code % dict.mod_ as u64) as usize;

    let mut remove_idx: Option<usize> = None;
    {
        let bucket = &dict.buckets[index];
        for (i, elem) in bucket.elements.iter().enumerate() {
            if elem.code != code {
                continue;
            }
            if keys_match(cmpr, &elem.key, key_data) {
                remove_idx = Some(i);
                break;
            }
        }
    }

    if let Some(i) = remove_idx {
        let mut elem = dict.buckets[index].elements.remove(i);
        if let Some(f) = dict.key.free {
            if dict.key.copy.is_some() {
                f(&mut elem.key);
            }
        }
        if dict.val.size != 0 {
            if let Some(f) = dict.val.free {
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
/// Check if a key exists in the dictionary. Matches C's dict_has(const dict_t*, ...).
pub fn dict_has(dict: &Dict, key_data: &[u8]) -> bool {
    let code = compute_hash_for(dict.key.type_, dict.key.hash, key_data);
    let cmpr = dict.key.cmpr;
    let index = (code % dict.mod_ as u64) as usize;

    for elem in &dict.buckets[index].elements {
        if elem.code != code {
            continue;
        }
        if keys_match(cmpr, &elem.key, key_data) {
            return true;
        }
    }
    false
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
    let total = dict.key.size.saturating_mul(*size);
    let mut bytes: Vec<u8> = Vec::with_capacity(total);
    for bucket in &dict.buckets {
        for elem in &bucket.elements {
            // Pad/truncate to the canonical key size.
            let mut padded = elem.key.clone();
            if padded.len() < dict.key.size {
                padded.resize(dict.key.size, 0);
            } else if padded.len() > dict.key.size {
                padded.truncate(dict.key.size);
            }
            bytes.extend_from_slice(&padded);
        }
    }
    let leaked: &'static [u8] = Box::leak(bytes.into_boxed_slice());
    Some(leaked)
}
/// Serialize a dictionary into a contiguous Vec<u8>. Matches C's dict_serialize().
pub fn dict_serialize(dict: &Dict, bytes: &mut usize) -> Option<Vec<u8>> {
    let count = dict_len(dict) as u32;
    let key_size = dict.key.size as u32;
    let val_size = dict.val.size as u32;

    let elem_size: usize = if dict.key.type_ == DictType::Str {
        4 + dict.val.size
    } else {
        dict.key.size + dict.val.size
    };

    let mut total: usize = 12 + (count as usize) * elem_size;

    // For STR keys, we also need to add up all the string lengths.
    let mut strlens: Vec<u32> = Vec::new();
    if dict.key.type_ == DictType::Str {
        strlens.reserve(count as usize);
        for bucket in &dict.buckets {
            for elem in &bucket.elements {
                let l = elem.key.len() as u32;
                strlens.push(l);
                total += l as usize;
            }
        }
    }

    *bytes = total;
    let mut data: Vec<u8> = Vec::with_capacity(total);

    // Header: [key_size, val_size, count]
    data.extend_from_slice(&key_size.to_ne_bytes());
    data.extend_from_slice(&val_size.to_ne_bytes());
    data.extend_from_slice(&count.to_ne_bytes());

    if dict.key.type_ == DictType::Str {
        // First write metadata, then write all strings concatenated
        let mut string_data: Vec<u8> = Vec::new();
        let mut idx = 0usize;
        for bucket in &dict.buckets {
            for elem in &bucket.elements {
                let l = strlens[idx];
                data.extend_from_slice(&l.to_ne_bytes());
                // Write the value bytes (padded to dict.val.size if needed)
                let mut v = elem.val.clone();
                v.resize(dict.val.size, 0);
                data.extend_from_slice(&v);
                string_data.extend_from_slice(&elem.key);
                idx += 1;
            }
        }
        data.extend_from_slice(&string_data);
    } else {
        for bucket in &dict.buckets {
            for elem in &bucket.elements {
                let mut k = elem.key.clone();
                k.resize(dict.key.size, 0);
                data.extend_from_slice(&k);
                let mut v = elem.val.clone();
                v.resize(dict.val.size, 0);
                data.extend_from_slice(&v);
            }
        }
    }

    Some(data)
}
/// Deserialize a dictionary from a slice. Matches C's dict_deserialize().
pub fn dict_deserialize(args: DictArgs, data: &[u8]) -> Dict {
    // Compute expected sizes for validation.
    let expected_key_size = natural_key_size(args.key.type_, args.key.size);
    let expected_val_size = align_size(args.val.size);

    // If the data is too small, return an empty dict.
    if data.len() < 12 {
        return dict_create(args);
    }

    let key_size_in = u32::from_ne_bytes(data[0..4].try_into().unwrap()) as usize;
    let val_size_in = u32::from_ne_bytes(data[4..8].try_into().unwrap()) as usize;
    let count_in = u32::from_ne_bytes(data[8..12].try_into().unwrap()) as usize;

    if key_size_in != expected_key_size || val_size_in != expected_val_size {
        // Size mismatch: return an empty dict (mirrors C's NULL return path).
        return dict_create(args);
    }

    let mut dict = dict_create(args);

    let mut ptr: usize = 12;
    if dict.key.type_ == DictType::Str {
        let elem_meta_size = 4 + val_size_in;
        let mut str_offset = ptr + count_in * elem_meta_size;
        for _ in 0..count_in {
            if ptr + 4 > data.len() {
                break;
            }
            let strlen = u32::from_ne_bytes(data[ptr..ptr + 4].try_into().unwrap()) as usize;
            ptr += 4;
            if ptr + val_size_in > data.len() {
                break;
            }
            let val = data[ptr..ptr + val_size_in].to_vec();
            ptr += val_size_in;
            if str_offset + strlen > data.len() {
                break;
            }
            let key = data[str_offset..str_offset + strlen].to_vec();
            str_offset += strlen;

            let code = compute_hash_for(dict.key.type_, dict.key.hash, &key);
            let index = (code % dict.mod_ as u64) as usize;
            dict.buckets[index].elements.push(DictElem { code, key, val });
            dict.count += 1;
        }
    } else {
        for _ in 0..count_in {
            if ptr + key_size_in + val_size_in > data.len() {
                break;
            }
            let key = data[ptr..ptr + key_size_in].to_vec();
            ptr += key_size_in;
            let val = data[ptr..ptr + val_size_in].to_vec();
            ptr += val_size_in;

            let code = compute_hash_for(dict.key.type_, dict.key.hash, &key);
            let index = (code % dict.mod_ as u64) as usize;
            dict.buckets[index].elements.push(DictElem { code, key, val });
            dict.count += 1;
        }
    }

    // Reshape if any bucket is "too crowded", mirroring C's behavior.
    let max_bucket = dict.buckets.iter().map(|b| b.elements.len()).max().unwrap_or(0);
    if max_bucket > DEFAULT_MOD {
        let factor = max_bucket / DEFAULT_MOD;
        dict_reshape(&mut dict, factor);
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
    keys_match(dict.key.cmpr, a, b)
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
    let new_size = old_size.saturating_mul(step).saturating_mul(DEFAULT_STEP);
    if new_size == 0 {
        return false;
    }

    let mut new_buckets: Vec<DictBucket> = (0..new_size)
        .map(|_| DictBucket { elements: Vec::new() })
        .collect();

    let old_buckets = std::mem::take(&mut dict.buckets);
    for bucket in old_buckets {
        for elem in bucket.elements {
            let index = (elem.code % new_size as u64) as usize;
            new_buckets[index].elements.push(elem);
        }
    }

    dict.buckets = new_buckets;
    dict.mod_ = new_size;
    true
}
/// Internal function to free a node. Matches C's dict_free_node().
pub fn dict_free_node(_dict: &Dict, _node: &mut DictElem) {
    // No-op: in safe Rust, the node is owned by the bucket Vec and freed automatically.
}
/// Internal function to free a dictionary key. Kept for signature consistency.
pub fn dict_free_key(dict: &Dict, key: &mut [u8]) {
    if dict.key.copy.is_some() {
        if let Some(f) = dict.key.free {
            f(key);
        }
    }
    // For DICT_STR in C, there's an explicit free call on the inner string pointer.
    // In the pure Rust version, the bytes are stored as Vec<u8> and freed automatically.
}
/// The original dict_get_hash. Kept for signature consistency but not used internally
/// to avoid borrow conflicts.
pub fn dict_get_hash(dict: &Dict, key: &[u8]) -> u64 {
    compute_hash_for(dict.key.type_, dict.key.hash, key)
}
