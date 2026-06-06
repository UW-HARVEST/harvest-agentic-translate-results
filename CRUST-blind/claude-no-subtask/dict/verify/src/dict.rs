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

// ---------------------------------------------------------------------------
// Internal helpers (private)
// ---------------------------------------------------------------------------

/// Round up `s` to the nearest multiple of `sizeof(uintptr_t)` (8 on 64-bit).
fn round_up_ptr(s: usize) -> usize {
    let p = std::mem::size_of::<usize>();
    (s + (p - 1)) & !(p - 1)
}

/// Compute the canonical key size for a given key type, mirroring the
/// switch statement in the C `dict_create()` function.
fn canonical_key_size(t: DictType, struct_size: usize) -> usize {
    match t {
        DictType::Char => 1,
        // wchar_t is 4 bytes on Linux/macOS (where the original C code is
        // expected to run). 4 bytes matches `int32_t` representation as
        // also used by va_arg in the original.
        DictType::WChar => 4,
        DictType::I32 | DictType::U32 | DictType::F32 => 4,
        DictType::I64 | DictType::U64 | DictType::F64 => 8,
        DictType::Ptr | DictType::Str => std::mem::size_of::<usize>(),
        DictType::Struct => round_up_ptr(struct_size),
    }
}

/// Prepare the key bytes that will be stored or used for lookup. For
/// fixed-size types we return a buffer of exactly `key.size` bytes,
/// padding with zeros if the input is shorter (and truncating if it is
/// longer). For variable-length string keys we just clone the bytes.
fn prepare_key_bytes(key_attr: &DictKeyAttr, key_data: &[u8]) -> Vec<u8> {
    match key_attr.type_ {
        DictType::Str => key_data.to_vec(),
        _ => {
            let mut buf = vec![0u8; key_attr.size];
            let n = key_data.len().min(key_attr.size);
            buf[..n].copy_from_slice(&key_data[..n]);
            buf
        }
    }
}

/// Compute the hash code for a normalized key. Mirrors C's `dict_get_hash`.
fn compute_hash_internal(key_attr: &DictKeyAttr, key_bytes: &[u8]) -> u64 {
    if let Some(h) = key_attr.hash {
        return h(key_bytes);
    }
    match key_attr.type_ {
        DictType::Char => {
            if key_bytes.is_empty() {
                0
            } else {
                // *(char*) key with implicit conversion to uint64_t -> sign extension
                key_bytes[0] as i8 as i64 as u64
            }
        }
        DictType::WChar | DictType::I32 => {
            let mut buf = [0u8; 4];
            let n = key_bytes.len().min(4);
            buf[..n].copy_from_slice(&key_bytes[..n]);
            i32::from_ne_bytes(buf) as i64 as u64
        }
        DictType::U32 => {
            let mut buf = [0u8; 4];
            let n = key_bytes.len().min(4);
            buf[..n].copy_from_slice(&key_bytes[..n]);
            u32::from_ne_bytes(buf) as u64
        }
        DictType::F32 => {
            let mut buf = [0u8; 4];
            let n = key_bytes.len().min(4);
            buf[..n].copy_from_slice(&key_bytes[..n]);
            // C: `code = *(float*) key` with implicit conversion to uint64_t.
            // Rust's `as u64` saturates negatives to 0 and NaN to 0 which is
            // close enough for typical numeric float keys.
            f32::from_ne_bytes(buf) as u64
        }
        DictType::I64 => {
            let mut buf = [0u8; 8];
            let n = key_bytes.len().min(8);
            buf[..n].copy_from_slice(&key_bytes[..n]);
            i64::from_ne_bytes(buf) as u64
        }
        DictType::U64 | DictType::Ptr => {
            let mut buf = [0u8; 8];
            let n = key_bytes.len().min(8);
            buf[..n].copy_from_slice(&key_bytes[..n]);
            u64::from_ne_bytes(buf)
        }
        DictType::F64 => {
            let mut buf = [0u8; 8];
            let n = key_bytes.len().min(8);
            buf[..n].copy_from_slice(&key_bytes[..n]);
            f64::from_ne_bytes(buf) as u64
        }
        DictType::Str | DictType::Struct => {
            // Rolling hash with base 256 mod 1e9+7. Bytes are interpreted
            // as `char`, which is signed on most platforms used by C tests.
            // For ASCII strings this is identical to using unsigned bytes,
            // so we use unsigned for predictability.
            let mut code: u64 = 0;
            for &b in key_bytes.iter() {
                code = (code.wrapping_mul(HASH_BASE).wrapping_add(b as u64)) % HASH_MOD;
            }
            code
        }
    }
}

/// Returns true if the two stored key byte vectors should be considered
/// equal. Mirrors the `cmpr`/strcmp/memcmp branches in C.
fn keys_equal_internal(key_attr: &DictKeyAttr, a: &[u8], b: &[u8]) -> bool {
    if let Some(c) = key_attr.cmpr {
        return c(a, b) == 0;
    }
    // For all built-in types (numeric, ptr, struct, str) byte-equality
    // matches the intent of the C code (memcmp / strcmp on the stored
    // representation).
    a == b
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Create a dictionary using detailed arguments. Matches C's dict_create().
pub fn dict_create(args: DictArgs) -> Dict {
    let key_size = canonical_key_size(args.key.type_, args.key.size);
    let val_size = round_up_ptr(args.val.size);

    let mut dict = Dict {
        key: args.key,
        val: args.val,
        alloc: args.alloc,
        mod_: DEFAULT_MOD,
        buckets: (0..DEFAULT_MOD)
            .map(|_| DictBucket {
                elements: Vec::new(),
            })
            .collect(),
        key_temp: vec![0u8; key_size],
        keys_dump: Vec::new(),
        count: 0,
    };
    dict.key.size = key_size;
    dict.val.size = val_size;
    dict
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
    let need_key_free = dict.key.copy.is_some() && dict.key.free.is_some();

    for bucket in &mut dict.buckets {
        for elem in &mut bucket.elements {
            if need_key_free {
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
    dict.count = 0;
}

/// Retrieve or create a value from the dictionary.
pub fn dict_get<'dict>(dict: &'dict mut Dict, key_data: &[u8]) -> Option<&'dict mut [u8]> {
    // Step 1: Prepare the final key bytes (no borrows beyond this scope).
    let key_attr = dict.key.clone();
    let val_size = dict.val.size;
    let key_bytes = prepare_key_bytes(&key_attr, key_data);

    // Step 2: Hash and bucket index.
    let code = compute_hash_internal(&key_attr, &key_bytes);
    let mod_ = dict.mod_;
    let index = (code % mod_ as u64) as usize;

    // Step 3: Search for an existing element.
    let found_pos = {
        let bucket = &dict.buckets[index];
        bucket.elements.iter().position(|e| {
            e.code == code && keys_equal_internal(&key_attr, &e.key, &key_bytes)
        })
    };

    let final_index;
    let final_pos;

    if let Some(pos) = found_pos {
        final_index = index;
        final_pos = pos;
    } else {
        // Step 4: Insert a new element. Keep a copy of key bytes for any
        // post-reshape lookup we need.
        let lookup_key = key_bytes.clone();
        let new_elem = DictElem {
            code,
            key: key_bytes,
            val: vec![0u8; val_size],
        };
        dict.buckets[index].elements.push(new_elem);
        dict.count += 1;
        let new_bucket_size = dict.buckets[index].elements.len();

        // Step 5: Possibly reshape (matches C's `size++ > mod` post-increment
        // semantics: trigger reshape when *previous* bucket size exceeded
        // the modulus).
        if new_bucket_size.saturating_sub(1) > dict.mod_ {
            if !dict_reshape(dict, 1) {
                return None;
            }
            // Step 6: After reshape, locate the inserted element in its
            // (possibly new) bucket position.
            let new_index = (code % dict.mod_ as u64) as usize;
            let new_pos = dict.buckets[new_index]
                .elements
                .iter()
                .position(|e| {
                    e.code == code && keys_equal_internal(&key_attr, &e.key, &lookup_key)
                })
                .unwrap_or(0);
            final_index = new_index;
            final_pos = new_pos;
        } else {
            final_index = index;
            final_pos = new_bucket_size - 1;
        }
    }

    Some(&mut dict.buckets[final_index].elements[final_pos].val)
}

/// Remove a value from the dictionary. Matches C's dict_remove(dict_t*, ...).
/// Returns true if the element was found and removed.
pub fn dict_remove(dict: &mut Dict, key_data: &[u8]) -> bool {
    let key_attr = dict.key.clone();
    let key_bytes = prepare_key_bytes(&key_attr, key_data);
    let code = compute_hash_internal(&key_attr, &key_bytes);
    let index = (code % dict.mod_ as u64) as usize;

    let pos = {
        let bucket = &dict.buckets[index];
        bucket.elements.iter().position(|e| {
            e.code == code && keys_equal_internal(&key_attr, &e.key, &key_bytes)
        })
    };

    if let Some(p) = pos {
        let key_free = dict.key.free;
        let val_free = dict.val.free;
        let val_size = dict.val.size;
        let need_key_free = dict.key.copy.is_some() && dict.key.free.is_some();

        let mut elem = dict.buckets[index].elements.remove(p);
        if need_key_free {
            if let Some(f) = key_free {
                f(&mut elem.key);
            }
        }
        if val_size != 0 {
            if let Some(f) = val_free {
                f(&mut elem.val);
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
    let key_attr = &dict.key;
    let key_bytes = prepare_key_bytes(key_attr, key_data);
    let code = compute_hash_internal(key_attr, &key_bytes);
    let index = (code % dict.mod_ as u64) as usize;

    dict.buckets[index]
        .elements
        .iter()
        .any(|e| e.code == code && keys_equal_internal(key_attr, &e.key, &key_bytes))
}

/// Return the number of elements in the dictionary. Matches C's dict_len().
pub fn dict_len(dict: &Dict) -> usize {
    let mut total = 0;
    for b in &dict.buckets {
        total += b.elements.len();
    }
    total
}

/// Return a snapshot of all keys.
pub fn dict_key(dict: &Dict, size: &mut usize) -> Option<&'static [u8]> {
    let count = dict_len(dict);
    *size = count;
    if count == 0 {
        return None;
    }

    let mut buf: Vec<u8> = Vec::new();

    if dict.key.type_ == DictType::Str {
        // For string keys we don't have stable C-style pointers, so we
        // simply concatenate the raw string bytes. Each string is
        // separated by a single null byte for parseability. This is a
        // best-effort emulation of the C return value.
        for bucket in &dict.buckets {
            for elem in &bucket.elements {
                buf.extend_from_slice(&elem.key);
                buf.push(0);
            }
        }
    } else {
        let ks = dict.key.size;
        for bucket in &dict.buckets {
            for elem in &bucket.elements {
                if elem.key.len() >= ks {
                    buf.extend_from_slice(&elem.key[..ks]);
                } else {
                    buf.extend_from_slice(&elem.key);
                    buf.resize(buf.len() + (ks - elem.key.len()), 0);
                }
            }
        }
    }

    let leaked: &'static [u8] = Box::leak(buf.into_boxed_slice());
    Some(leaked)
}

/// Serialize a dictionary into a contiguous Vec<u8>. Matches C's dict_serialize().
pub fn dict_serialize(dict: &Dict, bytes: &mut usize) -> Option<Vec<u8>> {
    let count = dict_len(dict);
    let key_size = dict.key.size;
    let val_size = dict.val.size;

    let elem_size: usize = if dict.key.type_ == DictType::Str {
        std::mem::size_of::<u32>() + val_size
    } else {
        key_size + val_size
    };

    let header_size = std::mem::size_of::<u32>() * 3;
    let mut total_bytes = header_size + count * elem_size;

    // For strings, also account for the variable-length string data.
    let mut strlen_table: Vec<u32> = Vec::new();
    if dict.key.type_ == DictType::Str {
        for bucket in &dict.buckets {
            for elem in &bucket.elements {
                let len = elem.key.len() as u32;
                strlen_table.push(len);
                total_bytes += len as usize;
            }
        }
    }

    let mut data = Vec::with_capacity(total_bytes);

    // Header: key_size, val_size, count (each u32, native byte order).
    data.extend_from_slice(&(key_size as u32).to_ne_bytes());
    data.extend_from_slice(&(val_size as u32).to_ne_bytes());
    data.extend_from_slice(&(count as u32).to_ne_bytes());

    if dict.key.type_ == DictType::Str {
        // First, all (strlen, val) records.
        let mut idx = 0;
        let mut string_data: Vec<u8> = Vec::new();
        for bucket in &dict.buckets {
            for elem in &bucket.elements {
                data.extend_from_slice(&strlen_table[idx].to_ne_bytes());
                if val_size > 0 {
                    if elem.val.len() >= val_size {
                        data.extend_from_slice(&elem.val[..val_size]);
                    } else {
                        data.extend_from_slice(&elem.val);
                        data.resize(data.len() + (val_size - elem.val.len()), 0);
                    }
                }
                string_data.extend_from_slice(&elem.key);
                idx += 1;
            }
        }
        data.extend_from_slice(&string_data);
    } else {
        for bucket in &dict.buckets {
            for elem in &bucket.elements {
                if elem.key.len() >= key_size {
                    data.extend_from_slice(&elem.key[..key_size]);
                } else {
                    data.extend_from_slice(&elem.key);
                    data.resize(data.len() + (key_size - elem.key.len()), 0);
                }
                if val_size > 0 {
                    if elem.val.len() >= val_size {
                        data.extend_from_slice(&elem.val[..val_size]);
                    } else {
                        data.extend_from_slice(&elem.val);
                        data.resize(data.len() + (val_size - elem.val.len()), 0);
                    }
                }
            }
        }
    }

    *bytes = total_bytes;
    Some(data)
}

/// Deserialize a dictionary from a slice. Matches C's dict_deserialize().
pub fn dict_deserialize(args: DictArgs, data: &[u8]) -> Dict {
    let header_sz = std::mem::size_of::<u32>() * 3;
    if data.len() < header_sz {
        // Not enough data; return an empty dict per `args`.
        return dict_create(args);
    }

    let read_u32_at = |off: usize| -> u32 {
        let mut buf = [0u8; 4];
        buf.copy_from_slice(&data[off..off + 4]);
        u32::from_ne_bytes(buf)
    };

    let header_key_size = read_u32_at(0) as usize;
    let header_val_size = read_u32_at(4) as usize;
    let count = read_u32_at(8) as usize;
    let mut ptr = header_sz;

    let computed_key_size = canonical_key_size(args.key.type_, args.key.size);
    let computed_val_size = round_up_ptr(args.val.size);

    if header_key_size != computed_key_size || header_val_size != computed_val_size {
        // Mismatch — return an empty (new) dictionary built from args.
        return dict_create(args);
    }

    // Build the dict shell.
    let mut dict = dict_create(args);
    let key_attr = dict.key.clone();

    if key_attr.type_ == DictType::Str {
        let elem_size = std::mem::size_of::<u32>() + header_val_size;
        let mut str_offset = ptr + count * elem_size;
        for _ in 0..count {
            if ptr + 4 > data.len() {
                break;
            }
            let strlen = read_u32_at(ptr) as usize;
            ptr += 4;
            let val_bytes = if header_val_size > 0 && ptr + header_val_size <= data.len() {
                let v = data[ptr..ptr + header_val_size].to_vec();
                ptr += header_val_size;
                v
            } else {
                vec![0u8; header_val_size]
            };
            let key_bytes = if str_offset + strlen <= data.len() {
                data[str_offset..str_offset + strlen].to_vec()
            } else {
                Vec::new()
            };
            str_offset += strlen;

            let code = compute_hash_internal(&key_attr, &key_bytes);
            let index = (code % dict.mod_ as u64) as usize;
            dict.buckets[index].elements.push(DictElem {
                code,
                key: key_bytes,
                val: val_bytes,
            });
            dict.count += 1;
        }
    } else {
        let key_size = header_key_size;
        let val_size = header_val_size;
        for _ in 0..count {
            if ptr + key_size + val_size > data.len() {
                break;
            }
            let key_bytes = data[ptr..ptr + key_size].to_vec();
            ptr += key_size;
            let val_bytes = if val_size > 0 {
                data[ptr..ptr + val_size].to_vec()
            } else {
                Vec::new()
            };
            ptr += val_size;

            let code = compute_hash_internal(&key_attr, &key_bytes);
            let index = (code % dict.mod_ as u64) as usize;
            dict.buckets[index].elements.push(DictElem {
                code,
                key: key_bytes,
                val: val_bytes,
            });
            dict.count += 1;
        }
    }

    // Possibly reshape if any bucket grew significantly.
    let max_bucket = dict
        .buckets
        .iter()
        .map(|b| b.elements.len())
        .max()
        .unwrap_or(0);
    if max_bucket > DEFAULT_MOD {
        let step = max_bucket / DEFAULT_MOD;
        let _ = dict_reshape(&mut dict, step);
    }

    dict
}

/// Convenience function to create a dictionary using inline arguments.
pub fn dict_create_args(args: DictArgs) -> Dict {
    dict_create(args)
}

/// Public helper retained from the original interface.
pub fn dict_key_equals(dict: &Dict, a: &[u8], b: &[u8]) -> bool {
    keys_equal_internal(&dict.key, a, b)
}

/// Not used in this design, but signature is kept.
pub fn dict_delete_node(_list: &mut DictBucket, _curr: &mut DictElem) {
    // no-op in this safe design
}

/// Call the value destructor, if present.
pub fn dict_free_val(dict: &Dict, val: &mut [u8]) {
    if dict.val.size != 0 {
        if let Some(f) = dict.val.free {
            f(val);
        }
    }
}

/// Not used in pure Rust version, matching signature only.
pub fn dict_get_key(_dict: &Dict) -> Option<&mut [u8]> {
    None
}

/// Internal function to reshape the dictionary. Matches C's dict_reshape().
pub fn dict_reshape(dict: &mut Dict, step: usize) -> bool {
    let old_size = dict.mod_;
    let new_size_opt = old_size
        .checked_mul(step)
        .and_then(|x| x.checked_mul(DEFAULT_STEP));
    let new_size = match new_size_opt {
        Some(s) if s > 0 => s,
        _ => return false,
    };

    let mut new_buckets: Vec<DictBucket> = (0..new_size)
        .map(|_| DictBucket {
            elements: Vec::new(),
        })
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

/// Free a node — no-op in pure-safe Rust because Vec frees memory itself.
pub fn dict_free_node(_dict: &Dict, _node: &mut DictElem) {
    // memory is reclaimed by Vec drop semantics
}

/// Free a dictionary key, if a custom destructor was provided.
pub fn dict_free_key(dict: &Dict, key: &mut [u8]) {
    if dict.key.copy.is_some() {
        if let Some(f) = dict.key.free {
            f(key);
        }
    }
}

/// Compute the hash of a key (helper retained from original interface).
pub fn dict_get_hash(dict: &Dict, key: &[u8]) -> u64 {
    compute_hash_internal(&dict.key, key)
}
