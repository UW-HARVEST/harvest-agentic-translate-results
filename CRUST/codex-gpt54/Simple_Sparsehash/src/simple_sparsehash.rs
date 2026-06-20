/// The maximum size of each sparse_array_group.
pub const GROUP_SIZE: usize = 48;
/// The default size of the hash table. Used to init bucket_max.
pub const STARTING_SIZE: usize = 32;
/// The default 'should we resize' percentage, out of 100.
pub const RESIZE_PERCENT: usize = 80;
/// Number of bits per u32.
pub const BITCHUNK_SIZE: usize = std::mem::size_of::<u32>() * 8;
/// The minimum number of u32 entries required to hold GROUP_SIZE bits.
pub const BITMAP_SIZE: usize = (GROUP_SIZE - 1) / BITCHUNK_SIZE + 1;
/// Represents one stored key/value pair.
#[derive(Debug)]
pub struct SparseBucket {
    /// The key as an owned String.
    pub key: String,
    /// The length of the key.
    pub klen: usize,
    /// The value as a vector of bytes.
    pub val: Vec<u8>,
    /// The length of the value.
    pub vlen: usize,
    /// The hash of the key.
    pub hash: u64,
}
/// One group in a sparse array.
#[derive(Debug)]
pub struct SparseArrayGroup {
    /// The number of items currently in this group.
    pub count: u32,
    /// The maximum size of each element.
    pub elem_size: usize,
    /// The storage for the elements.
    pub group: Vec<u8>,
    /// A bitmap tracking which slots in `group` are occupied.
    pub bitmap: [u32; BITMAP_SIZE],
}
/// A sparse array consisting of one or more groups.
#[derive(Debug)]
pub struct SparseArray {
    /// The maximum number of items that can be stored.
    pub maximum: usize,
    /// The groups that hold the elements.
    pub groups: Vec<SparseArrayGroup>,
}
/// A sparse dictionary that maps keys to values.
#[derive(Debug)]
pub struct SparseDict {
    /// The current maximum number of buckets in the dictionary.
    pub bucket_max: usize,
    /// The number of buckets that are currently occupied.
    pub bucket_count: usize,
    /// An array of sparse arrays (buckets).
    pub buckets: Vec<SparseArray>,
}

const U32_BITS: usize = u32::BITS as usize;
const USIZE_BYTES: usize = std::mem::size_of::<usize>();
const MIN_BUCKET_INLINE_SIZE: usize = 64;

fn max_arr_size(maximum: usize) -> usize {
    if maximum == 0 {
        0
    } else {
        (maximum - 1) / GROUP_SIZE + 1
    }
}

fn full_elem_size(elem_size: usize) -> usize {
    elem_size + USIZE_BYTES
}

fn charbit(position: usize) -> usize {
    position >> 5
}

fn modbit(position: usize) -> u32 {
    1u32 << (position & 31)
}

fn popcount_32(mut x: u32) -> u32 {
    const M1: u32 = 0x5555_5555;
    const M2: u32 = 0x3333_3333;
    const M4: u32 = 0x0f0f_0f0f;

    x -= (x >> 1) & M1;
    x = (x & M2) + ((x >> 2) & M2);
    x = (x + (x >> 4)) & M4;
    x += x >> 8;
    (x + (x >> 16)) & 0x3f
}

fn position_to_offset(bitmap: &[u32; BITMAP_SIZE], position: usize) -> usize {
    let mut retval = 0usize;
    let mut pos = position;
    let mut bitmap_iter = 0usize;

    while pos >= U32_BITS {
        retval += popcount_32(bitmap[bitmap_iter]) as usize;
        bitmap_iter += 1;
        pos -= U32_BITS;
    }

    retval + popcount_32(bitmap[bitmap_iter] & ((1u32 << pos) - 1u32)) as usize
}

fn is_position_occupied(bitmap: &[u32; BITMAP_SIZE], position: usize) -> bool {
    (bitmap[charbit(position)] & modbit(position)) != 0
}

fn set_position(bitmap: &mut [u32; BITMAP_SIZE], position: usize) {
    bitmap[charbit(position)] |= modbit(position);
}

fn hash_fnv1a(key: &str, klen: usize) -> u64 {
    const FNV_PRIME: u64 = 1_099_511_628_211;
    const FNV_OFFSET_BIAS: u64 = 14_695_981_039_346_656_037;

    let mut hash = FNV_OFFSET_BIAS;
    for &byte in &key.as_bytes()[..klen] {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    hash
}

fn quadratic_probe(key_hash: u64, num_probes: usize, maximum: usize) -> usize {
    ((key_hash as usize).wrapping_add(num_probes * num_probes)) & (maximum - 1)
}

fn serialize_bucket(bucket: &SparseBucket) -> Vec<u8> {
    let mut bytes =
        Vec::with_capacity(8 + (3 * USIZE_BYTES) + bucket.klen + bucket.vlen);
    bytes.extend_from_slice(&bucket.hash.to_ne_bytes());
    bytes.extend_from_slice(&bucket.klen.to_ne_bytes());
    bytes.extend_from_slice(&bucket.vlen.to_ne_bytes());
    bytes.extend_from_slice(&bucket.klen.to_ne_bytes());
    bytes.extend_from_slice(bucket.key.as_bytes());
    bytes.extend_from_slice(&bucket.val);
    bytes
}

fn parse_bucket_bytes(bytes: &[u8]) -> Option<(u64, usize, usize, &[u8], &[u8])> {
    let mut cursor = 0usize;

    let hash = u64::from_ne_bytes(bytes.get(cursor..cursor + 8)?.try_into().ok()?);
    cursor += 8;

    let klen = usize::from_ne_bytes(bytes.get(cursor..cursor + USIZE_BYTES)?.try_into().ok()?);
    cursor += USIZE_BYTES;

    let vlen = usize::from_ne_bytes(bytes.get(cursor..cursor + USIZE_BYTES)?.try_into().ok()?);
    cursor += USIZE_BYTES;

    let key_storage_len =
        usize::from_ne_bytes(bytes.get(cursor..cursor + USIZE_BYTES)?.try_into().ok()?);
    cursor += USIZE_BYTES;

    let key_bytes = bytes.get(cursor..cursor + key_storage_len)?;
    cursor += key_storage_len;
    let val = bytes.get(cursor..cursor + vlen)?;

    Some((hash, klen, vlen, key_bytes, val))
}

fn current_bucket_elem_size(dict: &SparseDict) -> usize {
    dict.buckets
        .first()
        .and_then(|array| array.groups.first())
        .map(|group| group.elem_size)
        .unwrap_or(MIN_BUCKET_INLINE_SIZE)
}

fn rehash_buckets(dict: &mut SparseDict, new_bucket_max: usize, new_elem_size: usize) -> i32 {
    let Some(mut new_buckets) = sparse_array_init(new_elem_size, new_bucket_max as u32) else {
        return 0;
    };

    let old_buckets = &dict.buckets[0];
    let mut buckets_rehashed = 0usize;

    for i in 0..dict.bucket_max {
        let mut bucket_siz = 0usize;
        let Some(bucket_bytes) = sparse_array_get(old_buckets, i as u32, Some(&mut bucket_siz)) else {
            continue;
        };

        if bucket_siz == 0 {
            continue;
        }

        let Some((hash, _, _, _, _)) = parse_bucket_bytes(bucket_bytes) else {
            return 0;
        };

        let mut num_probes = 0usize;
        loop {
            let probed_val = quadratic_probe(hash, num_probes, new_bucket_max);
            let mut current_value_siz = 0usize;
            let current_value =
                sparse_array_get(&new_buckets, probed_val as u32, Some(&mut current_value_siz));

            if current_value_siz == 0 && current_value.is_none() {
                if sparse_array_set(&mut new_buckets, probed_val as u32, bucket_bytes, bucket_siz) == 0 {
                    return 0;
                }
                break;
            }

            if num_probes > dict.bucket_count {
                return 0;
            }

            num_probes += 1;
        }

        buckets_rehashed += 1;
        if buckets_rehashed == dict.bucket_count {
            break;
        }
    }

    dict.buckets[0] = *new_buckets;
    dict.bucket_max = new_bucket_max;
    1
}

fn ensure_bucket_elem_size(dict: &mut SparseDict, required_size: usize) -> i32 {
    let current = current_bucket_elem_size(dict);
    if required_size <= current {
        return 1;
    }

    rehash_buckets(dict, dict.bucket_max, required_size.next_power_of_two())
}

fn sparse_array_group_set(
    arr: &mut SparseArrayGroup,
    i: usize,
    val: &[u8],
    vlen: usize,
) -> i32 {
    if vlen > arr.elem_size || vlen > val.len() {
        return 0;
    }

    let record_size = full_elem_size(arr.elem_size);
    let offset = position_to_offset(&arr.bitmap, i);

    if !is_position_occupied(&arr.bitmap, i) {
        let old_len = arr.group.len();
        arr.group.resize(old_len + record_size, 0);

        let move_bytes = (arr.count as usize - offset) * record_size;
        if move_bytes > 0 {
            arr.group
                .copy_within(offset * record_size..offset * record_size + move_bytes, (offset + 1) * record_size);
        }

        arr.count += 1;
        set_position(&mut arr.bitmap, i);
    }

    let start = offset * record_size;
    let end = start + record_size;
    let destination = &mut arr.group[start..end];
    destination[..USIZE_BYTES].copy_from_slice(&vlen.to_ne_bytes());
    destination[USIZE_BYTES..USIZE_BYTES + vlen].copy_from_slice(&val[..vlen]);

    1
}

fn sparse_array_group_get<'a>(
    arr: &'a SparseArrayGroup,
    i: usize,
    outsize: Option<&mut usize>,
) -> Option<&'a [u8]> {
    if !is_position_occupied(&arr.bitmap, i) {
        return None;
    }

    let record_size = full_elem_size(arr.elem_size);
    let offset = position_to_offset(&arr.bitmap, i);
    let start = offset * record_size;
    let size_bytes: [u8; USIZE_BYTES] = arr.group[start..start + USIZE_BYTES].try_into().ok()?;
    let item_size = usize::from_ne_bytes(size_bytes);

    if item_size == 0 {
        return None;
    }

    if let Some(outsize) = outsize {
        *outsize = item_size;
    }

    let value_start = start + USIZE_BYTES;
    Some(&arr.group[value_start..value_start + item_size])
}

/// Creates a new sparse array.
///
/// # Parameters
/// - `element_size`: Maximum size (in bytes) of each element.
/// - `maximum`: The maximum number of elements.
///
/// # Returns
/// An owned pointer (boxed) to a new `SparseArray` or `None` on failure.
pub fn sparse_array_init(element_size: usize, maximum: u32) -> Option<Box<SparseArray>> {
    let maximum = maximum as usize;
    let groups = (0..max_arr_size(maximum))
        .map(|_| SparseArrayGroup {
            count: 0,
            elem_size: element_size,
            group: Vec::new(),
            bitmap: [0; BITMAP_SIZE],
        })
        .collect();

    Some(Box::new(SparseArray { maximum, groups }))
}
/// Sets the element at index `i` to `val`.
///
/// # Parameters
/// - `arr`: The sparse array.
/// - `i`: The index at which to set the value.
/// - `val`: A slice of bytes holding the new element.
/// - `vlen`: The length of the value (in bytes).
///
/// # Returns
/// A nonzero integer on success and 0 on failure.
pub fn sparse_array_set(arr: &mut SparseArray, i: u32, val: &[u8], vlen: usize) -> i32 {
    let i = i as usize;
    if i > arr.maximum {
        return 0;
    }

    let group_index = i / GROUP_SIZE;
    let position = i % GROUP_SIZE;
    match arr.groups.get_mut(group_index) {
        Some(group) => sparse_array_group_set(group, position, val, vlen),
        None => 0,
    }
}
/// Retrieves the element at index `i`.
///
/// # Parameters
/// - `arr`: The sparse array.
/// - `i`: The index to retrieve.
/// - `outsize`: An optional mutable reference that will be set to the size (in bytes)
///   of the retrieved element.
///
/// # Returns
/// An optional slice reference to the element; `None` if the index is invalid.
pub fn sparse_array_get<'a>(
    arr: &'a SparseArray,
    i: u32,
    outsize: Option<&mut usize>,
) -> Option<&'a [u8]> {
    let i = i as usize;
    if i > arr.maximum {
        return None;
    }

    let group_index = i / GROUP_SIZE;
    let position = i % GROUP_SIZE;
    arr.groups
        .get(group_index)
        .and_then(|group| sparse_array_group_get(group, position, outsize))
}
/// Frees the sparse array.
///
/// # Parameters
/// - `arr`: The sparse array to free.
///
/// # Returns
/// A nonzero integer on success and 0 on failure.
pub fn sparse_array_free(arr: Box<SparseArray>) -> i32 {
    drop(arr);
    1
}
// ---------- Sparse Dictionary API ----------
/// Creates a new sparse dictionary.
///
/// # Returns
/// An owned pointer (boxed) to a new `SparseDict` or `None` on failure.
pub fn sparse_dict_init() -> Option<Box<SparseDict>> {
    let buckets = sparse_array_init(MIN_BUCKET_INLINE_SIZE, STARTING_SIZE as u32)?;
    Some(Box::new(SparseDict {
        bucket_max: STARTING_SIZE,
        bucket_count: 0,
        buckets: vec![*buckets],
    }))
}
/// Inserts a key/value pair into the dictionary.
///
/// # Parameters
/// - `dict`: The sparse dictionary.
/// - `key`: The key as a string slice.
/// - `klen`: The length of the key.
/// - `value`: A slice of bytes for the value.
/// - `vlen`: The length of the value.
///
/// # Returns
/// A nonzero integer on success and 0 on failure.
pub fn sparse_dict_set(
    dict: &mut SparseDict,
    key: &str,
    klen: usize,
    value: &[u8],
    vlen: usize,
) -> i32 {
    if dict.buckets.is_empty() || klen > key.len() || vlen > value.len() || dict.bucket_max == 0 {
        return 0;
    }

    let key_hash = hash_fnv1a(key, klen);
    let bucket = SparseBucket {
        key: key[..klen].to_owned(),
        klen,
        val: value[..vlen].to_vec(),
        vlen,
        hash: key_hash,
    };
    let serialized_bucket = serialize_bucket(&bucket);

    if ensure_bucket_elem_size(dict, serialized_bucket.len()) == 0 {
        return 0;
    }

    let mut num_probes = 0usize;

    loop {
        let probed_val = quadratic_probe(key_hash, num_probes, dict.bucket_max);
        let mut current_value_size = 0usize;
        let current_value =
            sparse_array_get(&dict.buckets[0], probed_val as u32, Some(&mut current_value_size));

        if current_value_size == 0 && current_value.is_none() {
            if sparse_array_set(
                &mut dict.buckets[0],
                probed_val as u32,
                &serialized_bucket,
                serialized_bucket.len(),
            ) == 0
            {
                return 0;
            }
            break;
        }

        if let Some(existing_bytes) = current_value {
            let Some((existing_hash, existing_klen, _, existing_key_bytes, _)) =
                parse_bucket_bytes(existing_bytes)
            else {
                return 0;
            };

            if existing_hash == key_hash
                && existing_klen == klen
                && existing_key_bytes == &key.as_bytes()[..klen]
            {
                return sparse_array_set(
                    &mut dict.buckets[0],
                    probed_val as u32,
                    &serialized_bucket,
                    serialized_bucket.len(),
                );
            }
        }

        num_probes += 1;
        if num_probes > dict.bucket_count {
            return 0;
        }
    }

    dict.bucket_count += 1;

    if (dict.bucket_count as f32) / (dict.bucket_max as f32) >= (RESIZE_PERCENT as f32) / 100.0 {
        return rehash_buckets(dict, dict.bucket_max * 2, current_bucket_elem_size(dict));
    }

    1
}
/// Retrieves the value associated with a key.
///
/// # Parameters
/// - `dict`: The sparse dictionary.
/// - `key`: The key as a string slice.
/// - `klen`: The length of the key.
/// - `outsize`: An optional mutable reference that will be set to the length of the value.
///
/// # Returns
/// An optional slice reference to the value; `None` if the key is not found.
pub fn sparse_dict_get<'a>(
    dict: &'a SparseDict,
    key: &str,
    klen: usize,
    outsize: Option<&mut usize>,
) -> Option<&'a [u8]> {
    if dict.buckets.is_empty() || klen > key.len() || dict.bucket_max == 0 {
        return None;
    }

    let key_hash = hash_fnv1a(key, klen);
    let mut num_probes = 0usize;

    loop {
        let probed_val = quadratic_probe(key_hash, num_probes, dict.bucket_max);
        let mut current_value_size = 0usize;
        let current_value =
            sparse_array_get(&dict.buckets[0], probed_val as u32, Some(&mut current_value_size));

        if let Some(current_value) = current_value {
            if current_value_size != 0 {
                let (hash, existing_klen, vlen, key_bytes, val) = parse_bucket_bytes(current_value)?;
                if hash == key_hash && existing_klen == klen && key_bytes == &key.as_bytes()[..klen] {
                    if let Some(outsize) = outsize {
                        *outsize = vlen;
                    }
                    return Some(val);
                }
            }
        } else {
            return None;
        }

        num_probes += 1;
        if num_probes > dict.bucket_count {
            return None;
        }
    }
}
/// Frees the sparse dictionary.
///
/// # Parameters
/// - `dict`: The sparse dictionary to free.
///
/// # Returns
/// A nonzero integer on success and 0 on failure.
pub fn sparse_dict_free(dict: Box<SparseDict>) -> i32 {
    drop(dict);
    1
}
