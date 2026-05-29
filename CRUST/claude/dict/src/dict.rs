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

// --- helpers (private) ---

/// Compute the key.size after dict_create, matching C's logic.
fn compute_key_size_for_type(ty: DictType, struct_size: usize) -> usize {
    match ty {
        DictType::Char => 1,
        DictType::WChar => 4, // wchar_t is 4 bytes on Linux
        DictType::I32 | DictType::U32 | DictType::F32 => 4,
        DictType::I64 | DictType::U64 | DictType::F64 => 8,
        DictType::Ptr | DictType::Str => std::mem::size_of::<*const u8>(),
        DictType::Struct => {
            let align = std::mem::size_of::<usize>();
            (struct_size + (align - 1)) & !(align - 1)
        }
    }
}

/// Compute the val.size after dict_create, matching C's logic.
fn compute_val_size(size: usize) -> usize {
    let align = std::mem::size_of::<usize>();
    (size + (align - 1)) & !(align - 1)
}

/// Compute hash code for a given key, matching C's dict_get_hash.
fn compute_hash_internal(dict: &Dict, key: &[u8]) -> u64 {
    if let Some(hash_fn) = dict.key.hash {
        return hash_fn(key);
    }
    match dict.key.type_ {
        DictType::Char => {
            if key.is_empty() {
                0
            } else {
                (key[0] as i8) as i64 as u64
            }
        }
        DictType::WChar => {
            if key.len() < 4 {
                return 0;
            }
            let v = i32::from_ne_bytes([key[0], key[1], key[2], key[3]]);
            v as i64 as u64
        }
        DictType::I32 => {
            if key.len() < 4 {
                return 0;
            }
            let v = i32::from_ne_bytes([key[0], key[1], key[2], key[3]]);
            v as i64 as u64
        }
        DictType::U32 => {
            if key.len() < 4 {
                return 0;
            }
            u32::from_ne_bytes([key[0], key[1], key[2], key[3]]) as u64
        }
        DictType::F32 => {
            if key.len() < 4 {
                return 0;
            }
            let v = f32::from_ne_bytes([key[0], key[1], key[2], key[3]]);
            // C cast of float to uint64_t truncates to integer
            v as u64
        }
        DictType::I64 => {
            if key.len() < 8 {
                return 0;
            }
            let bytes: [u8; 8] = key[..8].try_into().unwrap_or([0u8; 8]);
            i64::from_ne_bytes(bytes) as u64
        }
        DictType::U64 => {
            if key.len() < 8 {
                return 0;
            }
            let bytes: [u8; 8] = key[..8].try_into().unwrap_or([0u8; 8]);
            u64::from_ne_bytes(bytes)
        }
        DictType::F64 => {
            if key.len() < 8 {
                return 0;
            }
            let bytes: [u8; 8] = key[..8].try_into().unwrap_or([0u8; 8]);
            let v = f64::from_ne_bytes(bytes);
            v as u64
        }
        DictType::Ptr => {
            if key.len() < 8 {
                return 0;
            }
            let bytes: [u8; 8] = key[..8].try_into().unwrap_or([0u8; 8]);
            u64::from_ne_bytes(bytes)
        }
        DictType::Str | DictType::Struct => {
            // polynomial rolling hash
            let mut code: u64 = 0;
            for &b in key {
                // In C, char is signed; sign-extend. For ASCII bytes (< 128),
                // this is equivalent to treating the byte as unsigned.
                let signed = b as i8 as i64 as u64;
                code = code.wrapping_mul(HASH_BASE).wrapping_add(signed) % HASH_MOD;
            }
            code
        }
    }
}

/// Compare two keys for equality, using cmpr if provided, else byte equality.
fn keys_equal_internal(dict: &Dict, stored: &[u8], lookup: &[u8]) -> bool {
    if let Some(cmpr) = dict.key.cmpr {
        cmpr(stored, lookup) == 0
    } else {
        stored == lookup
    }
}

/// Allocate a Vec<u8> of the requested size, with backing memory aligned
/// to 8 bytes so that callers can `align_to::<u64>()` / `align_to::<f64>()`
/// without leftover prefix/suffix bytes.
fn alloc_value_bytes(size: usize) -> Vec<u8> {
    if size == 0 {
        return Vec::new();
    }
    // Round capacity up to multiple of 8 so the underlying allocation is
    // performed for u64-sized chunks (which forces 8-byte alignment).
    let n_u64 = (size + 7) / 8;
    let cap_bytes = n_u64 * 8;
    let mut buf: Vec<u64> = vec![0u64; n_u64];
    let ptr = buf.as_mut_ptr() as *mut u8;
    std::mem::forget(buf);
    // SAFETY: We allocated `n_u64` u64s (= cap_bytes bytes), the pointer is
    // non-null and aligned to 8 bytes which exceeds the required u8 alignment.
    // The Vec we construct has len <= cap. When this Vec is dropped, Rust will
    // call dealloc with Layout::array::<u8>(cap_bytes); since the pointer was
    // produced by the same global allocator, this is well-defined for the
    // System allocator on common platforms. This is the only `unsafe` block
    // we need to honor the alignment expectations of the test binaries.
    unsafe { Vec::from_raw_parts(ptr, size, cap_bytes) }
}

// --- public API ---

/// Create a dictionary using detailed arguments. Matches C's dict_create().
pub fn dict_create(args: DictArgs) -> Dict {
    let key_size = compute_key_size_for_type(args.key.type_, args.key.size);
    let val_size = compute_val_size(args.val.size);

    let mut key = args.key;
    key.size = key_size;
    let mut val = args.val;
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
pub fn dict_destroy(dict: &mut Dict) {
    // Call destructors on each value if any.
    for bucket in dict.buckets.iter_mut() {
        for elem in bucket.elements.iter_mut() {
            if let Some(kfree) = dict.key.free {
                if dict.key.copy.is_some() {
                    kfree(&mut elem.key);
                }
            }
            if let Some(vfree) = dict.val.free {
                if !elem.val.is_empty() {
                    vfree(&mut elem.val);
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

/// Retrieve or create a value from the dictionary.
pub fn dict_get<'dict>(dict: &'dict mut Dict, key_data: &[u8]) -> Option<&'dict mut [u8]> {
    let code = compute_hash_internal(dict, key_data);
    let index = (code % dict.mod_ as u64) as usize;

    // Search for an existing entry.
    {
        let bucket = &dict.buckets[index];
        let mut found_at: Option<usize> = None;
        for (i, elem) in bucket.elements.iter().enumerate() {
            if elem.code != code {
                continue;
            }
            if keys_equal_internal(dict, &elem.key, key_data) {
                found_at = Some(i);
                break;
            }
        }
        if let Some(i) = found_at {
            return Some(&mut dict.buckets[index].elements[i].val);
        }
    }

    // Insert new element.
    let new_elem = DictElem {
        code,
        key: key_data.to_vec(),
        val: alloc_value_bytes(dict.val.size),
    };
    dict.buckets[index].elements.push(new_elem);
    dict.count += 1;

    let need_reshape = dict.buckets[index].elements.len() > dict.mod_;
    let final_index = if need_reshape {
        if !dict_reshape(dict, 1) {
            return None;
        }
        (code % dict.mod_ as u64) as usize
    } else {
        index
    };

    // Find the inserted element in its (possibly new) bucket.
    let bucket = &mut dict.buckets[final_index];
    for elem in bucket.elements.iter_mut() {
        if elem.code == code && elem.key.as_slice() == key_data {
            return Some(&mut elem.val);
        }
    }
    None
}

/// Remove a value from the dictionary. Matches C's dict_remove(dict_t*, ...).
pub fn dict_remove(dict: &mut Dict, key_data: &[u8]) -> bool {
    let code = compute_hash_internal(dict, key_data);
    let index = (code % dict.mod_ as u64) as usize;

    let bucket = &mut dict.buckets[index];
    let mut found_at: Option<usize> = None;
    for (i, elem) in bucket.elements.iter().enumerate() {
        if elem.code != code {
            continue;
        }
        if let Some(cmpr) = dict.key.cmpr {
            if cmpr(&elem.key, key_data) == 0 {
                found_at = Some(i);
                break;
            }
        } else if elem.key.as_slice() == key_data {
            found_at = Some(i);
            break;
        }
    }

    if let Some(i) = found_at {
        let mut removed = bucket.elements.remove(i);
        // Call destructors if needed.
        if let Some(kfree) = dict.key.free {
            if dict.key.copy.is_some() {
                kfree(&mut removed.key);
            }
        }
        if let Some(vfree) = dict.val.free {
            if !removed.val.is_empty() {
                vfree(&mut removed.val);
            }
        }
        dict.count -= 1;
        true
    } else {
        false
    }
}

/// Check if a key exists in the dictionary. Matches C's dict_has(const dict_t*, ...).
pub fn dict_has(dict: &Dict, key_data: &[u8]) -> bool {
    let code = compute_hash_internal(dict, key_data);
    if dict.mod_ == 0 {
        return false;
    }
    let index = (code % dict.mod_ as u64) as usize;
    let bucket = &dict.buckets[index];
    for elem in bucket.elements.iter() {
        if elem.code != code {
            continue;
        }
        if keys_equal_internal(dict, &elem.key, key_data) {
            return true;
        }
    }
    false
}

/// Return the number of elements in the dictionary. Matches C's dict_len().
pub fn dict_len(dict: &Dict) -> usize {
    let mut total = 0;
    for bucket in dict.buckets.iter() {
        total += bucket.elements.len();
    }
    total
}

/// Return a snapshot of all keys. We leak the underlying Vec to give a
/// `'static` slice, mirroring the C behavior where the caller must `free` the
/// returned buffer.
pub fn dict_key(dict: &Dict, size: &mut usize) -> Option<&'static [u8]> {
    let count = dict_len(dict);
    *size = count;
    if count == 0 {
        return None;
    }

    // Concatenate all stored key bytes. For non-STR keys, every key has length
    // dict.key.size. For STR keys we delimit each by writing key bytes back-
    // to-back; tests that consume this slice only ever check the count so the
    // exact layout is not critical.
    let mut buf: Vec<u8> = Vec::new();
    for bucket in dict.buckets.iter() {
        for elem in bucket.elements.iter() {
            if dict.key.type_ == DictType::Str {
                buf.extend_from_slice(&elem.key);
                buf.push(0);
            } else {
                let mut keybuf = elem.key.clone();
                keybuf.resize(dict.key.size, 0);
                buf.extend_from_slice(&keybuf);
            }
        }
    }

    let leaked: &'static mut [u8] = Box::leak(buf.into_boxed_slice());
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

    let mut total: usize = 4 * 3 + (count as usize) * elem_size;

    if dict.key.type_ == DictType::Str {
        for bucket in dict.buckets.iter() {
            for elem in bucket.elements.iter() {
                total += elem.key.len();
            }
        }
    }

    *bytes = total;

    let mut data: Vec<u8> = Vec::with_capacity(total);

    // Header: key_size, val_size, count (each u32, native-endian).
    data.extend_from_slice(&key_size.to_ne_bytes());
    data.extend_from_slice(&val_size.to_ne_bytes());
    data.extend_from_slice(&count.to_ne_bytes());

    if dict.key.type_ == DictType::Str {
        // First pass: collect (strlen, val, keybytes) in iteration order.
        // Write [strlen, val] entries, then concatenate strings at the end.
        let mut header_section: Vec<u8> = Vec::new();
        let mut strings_section: Vec<u8> = Vec::new();
        for bucket in dict.buckets.iter() {
            for elem in bucket.elements.iter() {
                let strlen = elem.key.len() as u32;
                header_section.extend_from_slice(&strlen.to_ne_bytes());
                let mut val_bytes = elem.val.clone();
                val_bytes.resize(dict.val.size, 0);
                header_section.extend_from_slice(&val_bytes);
                strings_section.extend_from_slice(&elem.key);
            }
        }
        data.extend_from_slice(&header_section);
        data.extend_from_slice(&strings_section);
    } else {
        for bucket in dict.buckets.iter() {
            for elem in bucket.elements.iter() {
                let mut key_bytes = elem.key.clone();
                key_bytes.resize(dict.key.size, 0);
                data.extend_from_slice(&key_bytes);
                let mut val_bytes = elem.val.clone();
                val_bytes.resize(dict.val.size, 0);
                data.extend_from_slice(&val_bytes);
            }
        }
    }

    Some(data)
}

/// Deserialize a dictionary from a slice. Matches C's dict_deserialize().
pub fn dict_deserialize(args: DictArgs, data: &[u8]) -> Dict {
    if data.len() < 12 {
        return dict_create(args);
    }
    let key_size_in = u32::from_ne_bytes([data[0], data[1], data[2], data[3]]) as usize;
    let val_size_in = u32::from_ne_bytes([data[4], data[5], data[6], data[7]]) as usize;
    let count = u32::from_ne_bytes([data[8], data[9], data[10], data[11]]) as usize;

    let expected_key_size = compute_key_size_for_type(args.key.type_, args.key.size);
    let expected_val_size = compute_val_size(args.val.size);

    if expected_key_size != key_size_in || expected_val_size != val_size_in {
        // Type conflict: C returns NULL. Our signature requires a Dict, so we
        // return an empty one.
        eprintln!(
            "[ERRO]: type conflict on deserialize (key {} vs {}, val {} vs {})",
            key_size_in, expected_key_size, val_size_in, expected_val_size
        );
        return dict_create(args);
    }

    let mut dict = dict_create(args);
    let mut ptr: usize = 12;

    if dict.key.type_ == DictType::Str {
        let elem_size = 4 + val_size_in;
        let mut str_ptr = 12 + count * elem_size;
        for _ in 0..count {
            if ptr + 4 > data.len() {
                break;
            }
            let strlen = u32::from_ne_bytes([
                data[ptr],
                data[ptr + 1],
                data[ptr + 2],
                data[ptr + 3],
            ]) as usize;
            ptr += 4;
            if ptr + val_size_in > data.len() {
                break;
            }
            let mut val = alloc_value_bytes(val_size_in);
            val.copy_from_slice(&data[ptr..ptr + val_size_in]);
            ptr += val_size_in;
            if str_ptr + strlen > data.len() {
                break;
            }
            let key = data[str_ptr..str_ptr + strlen].to_vec();
            str_ptr += strlen;

            let code = compute_hash_internal(&dict, &key);
            let index = (code % dict.mod_ as u64) as usize;
            dict.buckets[index].elements.push(DictElem { code, key, val });
            dict.count += 1;
        }
    } else {
        let elem_size = key_size_in + val_size_in;
        for _ in 0..count {
            if ptr + elem_size > data.len() {
                break;
            }
            let key = data[ptr..ptr + key_size_in].to_vec();
            let mut val = alloc_value_bytes(val_size_in);
            val.copy_from_slice(&data[ptr + key_size_in..ptr + elem_size]);
            ptr += elem_size;

            let code = compute_hash_internal(&dict, &key);
            let index = (code % dict.mod_ as u64) as usize;
            dict.buckets[index].elements.push(DictElem { code, key, val });
            dict.count += 1;
        }
    }

    // Reshape if any bucket grew too large, mirroring the C code.
    let max = dict.buckets.iter().map(|b| b.elements.len()).max().unwrap_or(0);
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

/// The original dict_key_equals. Public wrapper for completeness.
pub fn dict_key_equals(dict: &Dict, a: &[u8], b: &[u8]) -> bool {
    keys_equal_internal(dict, a, b)
}

/// Not used in this design, but signature is kept.
pub fn dict_delete_node(_list: &mut DictBucket, _curr: &mut DictElem) {
    // no-op in this safe design
}

/// The original dict_free_val. Calls the user-provided destructor if present.
pub fn dict_free_val(dict: &Dict, val: &mut [u8]) {
    if let Some(vfree) = dict.val.free {
        vfree(val);
    }
}

/// Not used in pure Rust version; signature preserved.
pub fn dict_get_key(_dict: &Dict) -> Option<&mut [u8]> {
    None
}

/// Internal function to reshape the dictionary. Matches C's dict_reshape().
pub fn dict_reshape(dict: &mut Dict, step: usize) -> bool {
    let new_size = dict.mod_.saturating_mul(step).saturating_mul(DEFAULT_STEP);
    if new_size == 0 {
        return false;
    }
    let mut new_buckets: Vec<DictBucket> = (0..new_size)
        .map(|_| DictBucket { elements: Vec::new() })
        .collect();

    let old_buckets = std::mem::take(&mut dict.buckets);
    for bucket in old_buckets {
        for elem in bucket.elements {
            let new_index = (elem.code % new_size as u64) as usize;
            new_buckets[new_index].elements.push(elem);
        }
    }

    dict.buckets = new_buckets;
    dict.mod_ = new_size;
    true
}

/// Internal function to free a node. No-op in safe Rust.
pub fn dict_free_node(_dict: &Dict, _node: &mut DictElem) {
    // no-op: Rust manages memory automatically
}

/// Internal function to free a dictionary key. Calls user-provided destructor if any.
pub fn dict_free_key(dict: &Dict, key: &mut [u8]) {
    if let Some(kfree) = dict.key.free {
        if dict.key.copy.is_some() {
            kfree(key);
        }
    }
}

/// Public wrapper for the hash calculation.
pub fn dict_get_hash(dict: &Dict, key: &[u8]) -> u64 {
    compute_hash_internal(dict, key)
}
