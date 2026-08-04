/// A collection of constants matching the original C #defines.
pub const HASH_MOD: u64 = 1_000_000_007;
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
#[derive(Clone)]
pub struct DictElem {
    pub code: u64,
    pub key: Vec<u8>,
    pub val: Vec<u8>,
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

fn align_size(size: usize) -> usize {
    let align = core::mem::size_of::<usize>();
    (size + (align - 1)) & !(align - 1)
}

fn key_size_for_type(ty: DictType, struct_size: usize) -> usize {
    match ty {
        DictType::Char => core::mem::size_of::<i8>(),
        DictType::WChar => core::mem::size_of::<char>(),
        DictType::I32 => core::mem::size_of::<i32>(),
        DictType::U32 => core::mem::size_of::<u32>(),
        DictType::F32 => core::mem::size_of::<f32>(),
        DictType::I64 => core::mem::size_of::<i64>(),
        DictType::U64 => core::mem::size_of::<u64>(),
        DictType::F64 => core::mem::size_of::<f64>(),
        DictType::Ptr => core::mem::size_of::<usize>(),
        DictType::Str => core::mem::size_of::<usize>(),
        DictType::Struct => align_size(struct_size),
    }
}

fn read_padded<const N: usize>(data: &[u8]) -> [u8; N] {
    let mut out = [0u8; N];
    let used = data.len().min(N);
    out[..used].copy_from_slice(&data[..used]);
    out
}

fn normalize_key(dict: &Dict, key_data: &[u8]) -> Vec<u8> {
    if let Some(copy_fn) = dict.key.copy {
        let mut key = vec![0u8; dict.key.size];
        copy_fn(&mut key, key_data);
        return key;
    }

    match dict.key.type_ {
        DictType::Str => key_data.to_vec(),
        _ => {
            let mut key = vec![0u8; dict.key.size];
            let used = key.len().min(key_data.len());
            key[..used].copy_from_slice(&key_data[..used]);
            key
        }
    }
}

fn key_equals_impl(dict: &Dict, a: &[u8], b: &[u8]) -> bool {
    if let Some(cmpr) = dict.key.cmpr {
        return cmpr(a, b) == 0;
    }

    match dict.key.type_ {
        DictType::Str => a == b,
        _ => a == b,
    }
}

fn hash_impl(dict: &Dict, key: &[u8]) -> u64 {
    if let Some(hash) = dict.key.hash {
        return hash(key);
    }

    match dict.key.type_ {
        DictType::Char => i8::from_ne_bytes(read_padded::<1>(key)) as u64,
        DictType::WChar => u32::from_ne_bytes(read_padded::<4>(key)) as u64,
        DictType::I32 => i32::from_ne_bytes(read_padded::<4>(key)) as u64,
        DictType::U32 => u32::from_ne_bytes(read_padded::<4>(key)) as u64,
        DictType::F32 => f32::from_ne_bytes(read_padded::<4>(key)) as u64,
        DictType::I64 => i64::from_ne_bytes(read_padded::<8>(key)) as u64,
        DictType::U64 => u64::from_ne_bytes(read_padded::<8>(key)),
        DictType::F64 => f64::from_ne_bytes(read_padded::<8>(key)) as u64,
        DictType::Ptr => usize::from_ne_bytes(read_padded::<{ core::mem::size_of::<usize>() }>(key)) as u64,
        DictType::Str | DictType::Struct => key.iter().fold(0u64, |code, &byte| {
            (code * HASH_BASE + u64::from(byte)) % HASH_MOD
        }),
    }
}

fn empty_buckets(mod_: usize) -> Vec<DictBucket> {
    let mut buckets = Vec::with_capacity(mod_);
    for _ in 0..mod_ {
        buckets.push(DictBucket { elements: Vec::new() });
    }
    buckets
}

fn destroy_elem(dict: &Dict, elem: &mut DictElem) {
    dict_free_key(dict, &mut elem.key);
    dict_free_val(dict, &mut elem.val);
}

/// Create a dictionary using detailed arguments. Matches C's dict_create().
pub fn dict_create(args: DictArgs) -> Dict {
    let key_size = key_size_for_type(args.key.type_, args.key.size);
    let val_size = align_size(args.val.size);

    let mut key_attr = args.key.clone();
    key_attr.size = key_size;

    let mut val_attr = args.val.clone();
    val_attr.size = val_size;

    Dict {
        key: key_attr,
        val: val_attr,
        alloc: args.alloc,
        mod_: DEFAULT_MOD,
        buckets: empty_buckets(DEFAULT_MOD),
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
pub fn dict_destroy(dict: &mut Dict) {
    let mut buckets = core::mem::take(&mut dict.buckets);
    for bucket in &mut buckets {
        for elem in &mut bucket.elements {
            destroy_elem(dict, elem);
        }
        bucket.elements.clear();
    }
    dict.buckets = empty_buckets(dict.mod_);
    dict.count = 0;
    dict.key_temp.clear();
    dict.keys_dump.clear();
}

/// Retrieve or create a value from the dictionary. Matches C's dict_get(dict_t*, ...).
pub fn dict_get<'dict>(dict: &'dict mut Dict, key_data: &[u8]) -> Option<&'dict mut [u8]> {
    let key = normalize_key(dict, key_data);
    let code = hash_impl(dict, &key);
    let index = (code % dict.mod_ as u64) as usize;

    if let Some(pos) = dict.buckets[index]
        .elements
        .iter()
        .position(|elem| elem.code == code && key_equals_impl(dict, &elem.key, &key))
    {
        return Some(dict.buckets[index].elements[pos].val.as_mut_slice());
    }

    let previous_bucket_len = dict.buckets[index].elements.len();
    dict.buckets[index].elements.push(DictElem {
        code,
        key,
        val: vec![0u8; dict.val.size],
    });
    dict.count += 1;

    if previous_bucket_len > dict.mod_ && !dict_reshape(dict, 1) {
        return None;
    }

    let final_index = (code % dict.mod_ as u64) as usize;
    let pos = dict.buckets[final_index]
        .elements
        .iter()
        .position(|elem| elem.code == code && key_equals_impl(dict, &elem.key, key_data))
        .or_else(|| {
            dict.buckets[final_index]
                .elements
                .iter()
                .position(|elem| elem.code == code)
        })?;
    Some(dict.buckets[final_index].elements[pos].val.as_mut_slice())
}

/// Remove a value from the dictionary. Matches C's dict_remove(dict_t*, ...).
pub fn dict_remove(dict: &mut Dict, key_data: &[u8]) -> bool {
    let key = normalize_key(dict, key_data);
    let code = hash_impl(dict, &key);
    let index = (code % dict.mod_ as u64) as usize;

    let pos = match dict.buckets[index]
        .elements
        .iter()
        .position(|elem| elem.code == code && key_equals_impl(dict, &elem.key, &key))
    {
        Some(pos) => pos,
        None => return false,
    };

    let mut elem = dict.buckets[index].elements.remove(pos);
    destroy_elem(dict, &mut elem);
    dict.count -= 1;
    true
}

/// Check if a key exists in the dictionary. Matches C's dict_has(const dict_t*, ...).
pub fn dict_has(dict: &Dict, key_data: &[u8]) -> bool {
    let key = normalize_key(dict, key_data);
    let code = hash_impl(dict, &key);
    let index = (code % dict.mod_ as u64) as usize;

    dict.buckets[index]
        .elements
        .iter()
        .any(|elem| elem.code == code && key_equals_impl(dict, &elem.key, &key))
}

/// Return the number of elements in the dictionary. Matches C's dict_len().
pub fn dict_len(dict: &Dict) -> usize {
    dict.count
}

/// Return a snapshot of all keys.
pub fn dict_key(dict: &Dict, size: &mut usize) -> Option<&'static [u8]> {
    *size = dict_len(dict);
    if *size == 0 {
        return None;
    }

    let mut out = Vec::new();
    for bucket in &dict.buckets {
        for elem in &bucket.elements {
            match dict.key.type_ {
                DictType::Str => {
                    out.extend_from_slice(&elem.key);
                    out.push(0);
                }
                _ => out.extend_from_slice(&elem.key),
            }
        }
    }

    Some(Box::leak(out.into_boxed_slice()))
}

/// Serialize a dictionary into a contiguous Vec<u8>. Matches C's dict_serialize().
pub fn dict_serialize(dict: &Dict, bytes: &mut usize) -> Option<Vec<u8>> {
    let size = dict_len(dict);
    let count_u32 = u32::try_from(size).ok()?;
    let key_size_u32 = u32::try_from(dict.key.size).ok()?;
    let val_size_u32 = u32::try_from(dict.val.size).ok()?;

    let elem_size = if dict.key.type_ == DictType::Str {
        core::mem::size_of::<u32>() + dict.val.size
    } else {
        dict.key.size + dict.val.size
    };

    let mut total = core::mem::size_of::<u32>() * 3 + size * elem_size;
    let mut string_lengths = Vec::new();

    if dict.key.type_ == DictType::Str {
        for bucket in &dict.buckets {
            for elem in &bucket.elements {
                let len_u32 = u32::try_from(elem.key.len()).ok()?;
                total += elem.key.len();
                string_lengths.push(len_u32);
            }
        }
    }

    let mut data = Vec::with_capacity(total);
    data.extend_from_slice(&key_size_u32.to_ne_bytes());
    data.extend_from_slice(&val_size_u32.to_ne_bytes());
    data.extend_from_slice(&count_u32.to_ne_bytes());

    if dict.key.type_ == DictType::Str {
        let mut strings_blob = Vec::with_capacity(total - data.len() - size * elem_size);
        let mut str_index = 0usize;

        for bucket in &dict.buckets {
            for elem in &bucket.elements {
                data.extend_from_slice(&string_lengths[str_index].to_ne_bytes());
                data.extend_from_slice(&elem.val);
                strings_blob.extend_from_slice(&elem.key);
                str_index += 1;
            }
        }

        data.extend_from_slice(&strings_blob);
    } else {
        for bucket in &dict.buckets {
            for elem in &bucket.elements {
                data.extend_from_slice(&elem.key);
                data.extend_from_slice(&elem.val);
            }
        }
    }

    *bytes = data.len();
    Some(data)
}

/// Deserialize a dictionary from a slice. Matches C's dict_deserialize().
pub fn dict_deserialize(args: DictArgs, data: &[u8]) -> Dict {
    if data.len() < 12 {
        return dict_create(args);
    }

    let expected_key_size = key_size_for_type(args.key.type_, args.key.size);
    let expected_val_size = align_size(args.val.size);
    let stored_key_size = u32::from_ne_bytes(read_padded::<4>(&data[0..4])) as usize;
    let stored_val_size = u32::from_ne_bytes(read_padded::<4>(&data[4..8])) as usize;
    let count = u32::from_ne_bytes(read_padded::<4>(&data[8..12])) as usize;

    if stored_key_size != expected_key_size || stored_val_size != expected_val_size {
        return dict_create(args);
    }

    let mut dict = dict_create(args);
    let mut ptr = 12usize;

    if dict.key.type_ == DictType::Str {
        let elem_size = core::mem::size_of::<u32>() + dict.val.size;
        let strings_start = ptr.saturating_add(count.saturating_mul(elem_size));
        if strings_start > data.len() {
            return dict;
        }

        let mut str_ptr = strings_start;
        for _ in 0..count {
            if ptr + 4 > data.len() || ptr + 4 + dict.val.size > data.len() {
                break;
            }

            let str_len = u32::from_ne_bytes(read_padded::<4>(&data[ptr..ptr + 4])) as usize;
            ptr += 4;

            if str_ptr + str_len > data.len() {
                break;
            }

            let key = data[str_ptr..str_ptr + str_len].to_vec();
            str_ptr += str_len;

            let mut val = vec![0u8; dict.val.size];
            val.copy_from_slice(&data[ptr..ptr + dict.val.size]);
            ptr += dict.val.size;

            let code = hash_impl(&dict, &key);
            let index = (code % dict.mod_ as u64) as usize;
            dict.buckets[index].elements.push(DictElem { code, key, val });
            dict.count += 1;
        }
    } else {
        let elem_size = dict.key.size + dict.val.size;
        for _ in 0..count {
            if ptr + elem_size > data.len() {
                break;
            }

            let key = data[ptr..ptr + dict.key.size].to_vec();
            let val = data[ptr + dict.key.size..ptr + elem_size].to_vec();
            ptr += elem_size;

            let code = hash_impl(&dict, &key);
            let index = (code % dict.mod_ as u64) as usize;
            dict.buckets[index].elements.push(DictElem { code, key, val });
            dict.count += 1;
        }
    }

    let max_bucket_len = dict
        .buckets
        .iter()
        .map(|bucket| bucket.elements.len())
        .max()
        .unwrap_or(0);

    if max_bucket_len > DEFAULT_MOD {
        let step = max_bucket_len / DEFAULT_MOD;
        let _ = dict_reshape(&mut dict, step);
    }

    dict
}

/// Convenience function to create a dictionary using inline arguments.
pub fn dict_create_args(args: DictArgs) -> Dict {
    dict_create(args)
}

/// The original dict_key_equals.
pub fn dict_key_equals(dict: &Dict, a: &[u8], b: &[u8]) -> bool {
    key_equals_impl(dict, a, b)
}

/// Not used in this design, but signature is kept.
pub fn dict_delete_node(_list: &mut DictBucket, _curr: &mut DictElem) {}

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

/// Internal function to reshape the dictionary.
pub fn dict_reshape(dict: &mut Dict, step: usize) -> bool {
    let new_mod = dict.mod_.saturating_mul(step).saturating_mul(DEFAULT_STEP);
    if new_mod == 0 {
        return false;
    }

    let mut new_buckets = empty_buckets(new_mod);
    for bucket in core::mem::take(&mut dict.buckets) {
        for elem in bucket.elements {
            let index = (elem.code % new_mod as u64) as usize;
            new_buckets[index].elements.push(elem);
        }
    }

    dict.mod_ = new_mod;
    dict.buckets = new_buckets;
    true
}

/// Internal function to free a node. Matches C's dict_free_node().
pub fn dict_free_node(_dict: &Dict, _node: &mut DictElem) {}

/// Internal function to free a dictionary key. Kept for signature consistency.
pub fn dict_free_key(dict: &Dict, key: &mut [u8]) {
    if let Some(free_fn) = dict.key.free {
        free_fn(key);
    }
}

/// The original dict_get_hash.
pub fn dict_get_hash(dict: &Dict, key: &[u8]) -> u64 {
    hash_impl(dict, key)
}
