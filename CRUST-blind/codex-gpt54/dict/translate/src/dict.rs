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

fn align_up(size: usize) -> usize {
    let align = std::mem::size_of::<usize>();
    (size + (align - 1)) & !(align - 1)
}

fn derived_key_size(key_type: DictType, key_size: usize) -> usize {
    match key_type {
        DictType::Char => std::mem::size_of::<i8>(),
        DictType::WChar => std::mem::size_of::<char>(),
        DictType::I32 => std::mem::size_of::<i32>(),
        DictType::U32 => std::mem::size_of::<u32>(),
        DictType::F32 => std::mem::size_of::<f32>(),
        DictType::I64 => std::mem::size_of::<i64>(),
        DictType::U64 => std::mem::size_of::<u64>(),
        DictType::F64 => std::mem::size_of::<f64>(),
        DictType::Ptr => std::mem::size_of::<usize>(),
        DictType::Str => std::mem::size_of::<usize>(),
        DictType::Struct => align_up(key_size),
    }
}

fn copy_into_sized_buffer(src: &[u8], size: usize) -> Vec<u8> {
    let mut out = vec![0; size];
    let len = src.len().min(size);
    out[..len].copy_from_slice(&src[..len]);
    out
}

fn read_array<const N: usize>(data: &[u8]) -> [u8; N] {
    let mut out = [0u8; N];
    let len = data.len().min(N);
    out[..len].copy_from_slice(&data[..len]);
    out
}

fn normalize_key_input(dict: &Dict, key_data: &[u8]) -> Vec<u8> {
    if let Some(copy) = dict.key.copy {
        let mut key = vec![0; dict.key.size];
        copy(&mut key, key_data);
        return key;
    }

    match dict.key.type_ {
        DictType::Str => {
            let end = key_data.iter().position(|byte| *byte == 0).unwrap_or(key_data.len());
            key_data[..end].to_vec()
        }
        _ => copy_into_sized_buffer(key_data, dict.key.size),
    }
}

fn key_matches(dict: &Dict, existing: &[u8], candidate: &[u8]) -> bool {
    if let Some(cmpr) = dict.key.cmpr {
        cmpr(existing, candidate) == 0
    } else {
        match dict.key.type_ {
            DictType::Str => existing == candidate,
            _ => existing == candidate,
        }
    }
}

fn find_element_index(dict: &Dict, index: usize, code: u64, key: &[u8]) -> Option<usize> {
    dict.buckets
        .get(index)?
        .elements
        .iter()
        .position(|elem| elem.code == code && key_matches(dict, &elem.key, key))
}

fn push_elem(dict: &mut Dict, index: usize, elem: DictElem) {
    dict.buckets[index].elements.push(elem);
    dict.count += 1;
}
/// Create a dictionary using detailed arguments. Matches C's dict_create().
pub fn dict_create(args: DictArgs) -> Dict {
    let key_size = derived_key_size(args.key.type_, args.key.size);
    let val_size = align_up(args.val.size);

    Dict {
        key: DictKeyAttr {
            type_: args.key.type_,
            size: key_size,
            copy: args.key.copy,
            free: args.key.free,
            hash: args.key.hash,
            cmpr: args.key.cmpr,
        },
        val: DictValAttr {
            size: val_size,
            free: args.val.free,
        },
        alloc: args.alloc,
        mod_: DEFAULT_MOD,
        buckets: vec![
            DictBucket {
                elements: Vec::new(),
            };
            DEFAULT_MOD
        ],
        key_temp: vec![0; key_size],
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
    let key_free = if dict.key.copy.is_some() { dict.key.free } else { None };
    let val_free = dict.val.free;

    for bucket in &mut dict.buckets {
        for elem in &mut bucket.elements {
            if let Some(free) = key_free {
                free(&mut elem.key);
            }
            if !elem.val.is_empty() {
                if let Some(free) = val_free {
                    free(&mut elem.val);
                }
            }
        }
        bucket.elements.clear();
    }
    dict.count = 0;
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
    let key = normalize_key_input(dict, key_data);
    let code = dict_get_hash(dict, &key);
    let index = (code % dict.mod_ as u64) as usize;

    if let Some(pos) = find_element_index(dict, index, code, &key) {
        return Some(dict.buckets[index].elements[pos].val.as_mut_slice());
    }

    let should_reshape = dict.buckets[index].elements.len() > dict.mod_;
    push_elem(
        dict,
        index,
        DictElem {
            code,
            key,
            val: vec![0; dict.val.size],
        },
    );

    if should_reshape && !dict_reshape(dict, 1) {
        return None;
    }

    let final_index = (code % dict.mod_ as u64) as usize;
    let pos = find_element_index(dict, final_index, code, key_data)
        .or_else(|| find_element_index(dict, final_index, code, &normalize_key_input(dict, key_data)))?;
    Some(dict.buckets[final_index].elements[pos].val.as_mut_slice())
}
/// Remove a value from the dictionary. Matches C's dict_remove(dict_t*, ...).
/// Returns true if the element was found and removed.
pub fn dict_remove(dict: &mut Dict, key_data: &[u8]) -> bool {
    let key = normalize_key_input(dict, key_data);
    let code = dict_get_hash(dict, &key);
    let index = (code % dict.mod_ as u64) as usize;

    let Some(pos) = find_element_index(dict, index, code, &key) else {
        return false;
    };

    let mut elem = dict.buckets[index].elements.remove(pos);
    dict.count = dict.count.saturating_sub(1);
    dict_free_key(dict, &mut elem.key);
    dict_free_val(dict, &mut elem.val);
    true
}
/// Check if a key exists in the dictionary. Matches C's dict_has(const dict_t*, ...).
pub fn dict_has(dict: &Dict, key_data: &[u8]) -> bool {
    let key = normalize_key_input(dict, key_data);
    let code = dict_get_hash(dict, &key);
    let index = (code % dict.mod_ as u64) as usize;
    find_element_index(dict, index, code, &key).is_some()
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
    *size = dict_len(dict);
    if *size == 0 {
        return None;
    }

    let mut out = Vec::new();
    match dict.key.type_ {
        DictType::Str => {
            for bucket in &dict.buckets {
                for elem in &bucket.elements {
                    out.extend_from_slice(&elem.key);
                    out.push(0);
                }
            }
        }
        _ => {
            out.reserve(dict.key.size * *size);
            for bucket in &dict.buckets {
                for elem in &bucket.elements {
                    out.extend_from_slice(&elem.key);
                }
            }
        }
    }

    Some(Box::leak(out.into_boxed_slice()))
}
/// Serialize a dictionary into a contiguous Vec<u8>. Matches C's dict_serialize().
pub fn dict_serialize(dict: &Dict, bytes: &mut usize) -> Option<Vec<u8>> {
    let size = dict_len(dict);
    let header_len = std::mem::size_of::<u32>() * 3;
    let elem_size = if dict.key.type_ == DictType::Str {
        std::mem::size_of::<u32>() + dict.val.size
    } else {
        dict.key.size + dict.val.size
    };

    let strings_size = if dict.key.type_ == DictType::Str {
        dict.buckets
            .iter()
            .flat_map(|bucket| bucket.elements.iter())
            .map(|elem| elem.key.len())
            .sum::<usize>()
    } else {
        0
    };

    *bytes = header_len + (size * elem_size) + strings_size;
    let mut data = Vec::with_capacity(*bytes);
    data.extend_from_slice(&(dict.key.size as u32).to_ne_bytes());
    data.extend_from_slice(&(dict.val.size as u32).to_ne_bytes());
    data.extend_from_slice(&(size as u32).to_ne_bytes());

    if dict.key.type_ == DictType::Str {
        let mut strings = Vec::with_capacity(strings_size);
        for bucket in &dict.buckets {
            for elem in &bucket.elements {
                data.extend_from_slice(&(elem.key.len() as u32).to_ne_bytes());
                data.extend_from_slice(&elem.val);
                strings.extend_from_slice(&elem.key);
            }
        }
        data.extend_from_slice(&strings);
    } else {
        for bucket in &dict.buckets {
            for elem in &bucket.elements {
                data.extend_from_slice(&elem.key);
                data.extend_from_slice(&elem.val);
            }
        }
    }

    Some(data)
}
/// Deserialize a dictionary from a slice. Matches C's dict_deserialize().
pub fn dict_deserialize(args: DictArgs, data: &[u8]) -> Dict {
    if data.len() < std::mem::size_of::<u32>() * 3 {
        return dict_create(args);
    }

    let expected_key_size = derived_key_size(args.key.type_, args.key.size);
    let expected_val_size = align_up(args.val.size);

    let key_size = u32::from_ne_bytes(read_array::<4>(&data[0..4])) as usize;
    let val_size = u32::from_ne_bytes(read_array::<4>(&data[4..8])) as usize;
    let count = u32::from_ne_bytes(read_array::<4>(&data[8..12])) as usize;

    if key_size != expected_key_size || val_size != expected_val_size {
        return dict_create(args);
    }

    let mut dict = dict_create(args);
    let mut ptr = 12usize;
    let elem_size = if dict.key.type_ == DictType::Str {
        std::mem::size_of::<u32>() + dict.val.size
    } else {
        dict.key.size + dict.val.size
    };

    if dict.key.type_ == DictType::Str {
        let mut str_ptr = ptr.saturating_add(count.saturating_mul(elem_size));
        for _ in 0..count {
            if ptr + 4 > data.len() {
                break;
            }
            let str_len = u32::from_ne_bytes(read_array::<4>(&data[ptr..ptr + 4])) as usize;
            ptr += 4;
            if ptr + dict.val.size > data.len() || str_ptr + str_len > data.len() {
                break;
            }
            let val = data[ptr..ptr + dict.val.size].to_vec();
            ptr += dict.val.size;
            let key = data[str_ptr..str_ptr + str_len].to_vec();
            str_ptr += str_len;
            let code = dict_get_hash(&dict, &key);
            let index = (code % dict.mod_ as u64) as usize;
            push_elem(&mut dict, index, DictElem { code, key, val });
        }
    } else {
        for _ in 0..count {
            if ptr + elem_size > data.len() {
                break;
            }
            let key = data[ptr..ptr + dict.key.size].to_vec();
            let val = data[ptr + dict.key.size..ptr + elem_size].to_vec();
            ptr += elem_size;
            let code = dict_get_hash(&dict, &key);
            let index = (code % dict.mod_ as u64) as usize;
            push_elem(&mut dict, index, DictElem { code, key, val });
        }
    }

    let max_bucket = dict
        .buckets
        .iter()
        .map(|bucket| bucket.elements.len())
        .max()
        .unwrap_or(0);
    if max_bucket > DEFAULT_MOD {
        let _ = dict_reshape(&mut dict, max_bucket / DEFAULT_MOD);
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
    key_matches(dict, a, b)
}
/// Not used in this design, but signature is kept.
pub fn dict_delete_node(_list: &mut DictBucket, _curr: &mut DictElem) {
// no-op in this safe design
}
/// The original dict_free_val. Kept for signature consistency.
pub fn dict_free_val(dict: &Dict, val: &mut [u8]) {
    if let Some(free) = dict.val.free {
        free(val);
    }
}
/// Not used in pure Rust version, matching signature only.
pub fn dict_get_key(_dict: &Dict) -> Option<&mut [u8]> {
    None
}
/// Internal function to reshape the dictionary. Matches C's dict_reshape().
/// We re-allocate and re-hash all elements with new capacity = old * step * DEFAULT_STEP.
pub fn dict_reshape(dict: &mut Dict, step: usize) -> bool {
    if step == 0 {
        return false;
    }
    let Some(new_size) = dict
        .mod_
        .checked_mul(step)
        .and_then(|value| value.checked_mul(DEFAULT_STEP))
    else {
        return false;
    };

    let old_buckets = std::mem::replace(
        &mut dict.buckets,
        vec![
            DictBucket {
                elements: Vec::new(),
            };
            new_size
        ],
    );
    dict.mod_ = new_size;

    for bucket in old_buckets {
        for elem in bucket.elements {
            let index = (elem.code % dict.mod_ as u64) as usize;
            dict.buckets[index].elements.push(elem);
        }
    }

    true
}
/// Internal function to free a node. Matches C's dict_free_node().
pub fn dict_free_node(_dict: &Dict, _node: &mut DictElem) {
    // no-op: dropping the element releases its owned buffers
}
/// Internal function to free a dictionary key. Kept for signature consistency.
pub fn dict_free_key(dict: &Dict, key: &mut [u8]) {
    if dict.key.copy.is_some() {
        if let Some(free) = dict.key.free {
            free(key);
        }
    }
}
/// The original dict_get_hash. Kept for signature consistency but not used internally
/// to avoid borrow conflicts.
pub fn dict_get_hash(dict: &Dict, key: &[u8]) -> u64 {
    if let Some(hash) = dict.key.hash {
        return hash(key);
    }

    match dict.key.type_ {
        DictType::Char => i8::from_ne_bytes(read_array::<1>(key)) as u64,
        DictType::WChar => u32::from_ne_bytes(read_array::<4>(key)) as u64,
        DictType::I32 => i32::from_ne_bytes(read_array::<4>(key)) as u64,
        DictType::U32 => u32::from_ne_bytes(read_array::<4>(key)) as u64,
        DictType::F32 => f32::from_ne_bytes(read_array::<4>(key)) as u64,
        DictType::I64 => i64::from_ne_bytes(read_array::<8>(key)) as u64,
        DictType::U64 => u64::from_ne_bytes(read_array::<8>(key)),
        DictType::F64 => f64::from_ne_bytes(read_array::<8>(key)) as u64,
        DictType::Ptr => usize::from_ne_bytes(read_array::<{ std::mem::size_of::<usize>() }>(key)) as u64,
        DictType::Str | DictType::Struct => key.iter().fold(0u64, |code, byte| {
            (code * HASH_BASE + (*byte as u64)) % HASH_MOD
        }),
    }
}
