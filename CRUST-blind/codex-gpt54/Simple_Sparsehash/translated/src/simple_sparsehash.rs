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

const USIZE_BYTES: usize = std::mem::size_of::<usize>();
const U64_BYTES: usize = std::mem::size_of::<u64>();
const BUCKET_HEADER_SIZE: usize = U64_BYTES + (USIZE_BYTES * 2);

fn full_elem_size(elem_size: usize) -> usize {
    elem_size + USIZE_BYTES
}

fn max_arr_size(maximum: usize) -> usize {
    maximum.saturating_sub(1) / GROUP_SIZE + 1
}

fn charbit(position: u32) -> usize {
    (position >> 5) as usize
}

fn modbit(position: u32) -> u32 {
    1u32 << (position & 31)
}

fn popcount_32(x: u32) -> u32 {
    x.count_ones()
}

fn position_to_offset(bitmap: &[u32; BITMAP_SIZE], position: u32) -> usize {
    let mut retval = 0u32;
    let mut pos = position;
    let mut bitmap_iter = 0usize;

    while pos >= BITCHUNK_SIZE as u32 {
        retval += popcount_32(bitmap[bitmap_iter]);
        bitmap_iter += 1;
        pos -= BITCHUNK_SIZE as u32;
    }

    let remainder_mask = if pos == 0 { 0 } else { (1u32 << pos) - 1u32 };
    (retval + popcount_32(bitmap[bitmap_iter] & remainder_mask)) as usize
}

fn is_position_occupied(bitmap: &[u32; BITMAP_SIZE], position: u32) -> bool {
    (bitmap[charbit(position)] & modbit(position)) != 0
}

fn set_position(bitmap: &mut [u32; BITMAP_SIZE], position: u32) {
    bitmap[charbit(position)] |= modbit(position);
}

fn read_usize(bytes: &[u8]) -> Option<usize> {
    let raw: [u8; USIZE_BYTES] = bytes.get(..USIZE_BYTES)?.try_into().ok()?;
    Some(usize::from_ne_bytes(raw))
}

fn write_usize(dst: &mut [u8], value: usize) -> bool {
    if dst.len() < USIZE_BYTES {
        return false;
    }
    dst[..USIZE_BYTES].copy_from_slice(&value.to_ne_bytes());
    true
}

fn hash_fnv1a(key: &[u8]) -> u64 {
    const FNV_PRIME: u64 = 1099511628211;
    const FNV_OFFSET_BASIS: u64 = 14695981039346656037;

    let mut hash = FNV_OFFSET_BASIS;
    for &byte in key {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    hash
}

fn quadratic_probe(key_hash: u64, num_probes: usize, maximum: usize) -> usize {
    let probe = (num_probes as u64).wrapping_mul(num_probes as u64);
    key_hash.wrapping_add(probe) as usize & (maximum - 1)
}

fn empty_bucket_slot() -> SparseArray {
    SparseArray {
        maximum: 0,
        groups: Vec::new(),
    }
}

fn bucket_slot_is_empty(slot: &SparseArray) -> bool {
    slot.groups.first().is_none_or(|group| group.group.is_empty())
}

fn build_bucket_slot(hash: u64, key: &[u8], value: &[u8]) -> SparseArray {
    let mut data = Vec::with_capacity(BUCKET_HEADER_SIZE + key.len() + value.len());
    data.extend_from_slice(&hash.to_ne_bytes());
    data.extend_from_slice(&key.len().to_ne_bytes());
    data.extend_from_slice(&value.len().to_ne_bytes());
    data.extend_from_slice(key);
    data.extend_from_slice(value);

    SparseArray {
        maximum: 1,
        groups: vec![SparseArrayGroup {
            count: 1,
            elem_size: data.len(),
            group: data,
            bitmap: [0; BITMAP_SIZE],
        }],
    }
}

fn parse_bucket_slot(slot: &SparseArray) -> Option<(u64, &[u8], &[u8])> {
    let group = slot.groups.first()?;
    let data = &group.group;
    let hash_bytes: [u8; U64_BYTES] = data.get(..U64_BYTES)?.try_into().ok()?;
    let hash = u64::from_ne_bytes(hash_bytes);
    let klen = read_usize(data.get(U64_BYTES..U64_BYTES + USIZE_BYTES)?)?;
    let vlen = read_usize(data.get(U64_BYTES + USIZE_BYTES..BUCKET_HEADER_SIZE)?)?;
    let key_start = BUCKET_HEADER_SIZE;
    let value_start = key_start.checked_add(klen)?;
    let end = value_start.checked_add(vlen)?;
    let key = data.get(key_start..value_start)?;
    let value = data.get(value_start..end)?;
    Some((hash, key, value))
}

fn sparse_array_group_set(
    arr: &mut SparseArrayGroup,
    i: u32,
    val: &[u8],
    vlen: usize,
) -> i32 {
    if vlen > arr.elem_size || val.len() < vlen {
        return 0;
    }

    let offset = position_to_offset(&arr.bitmap, i);
    let full_size = full_elem_size(arr.elem_size);

    if !is_position_occupied(&arr.bitmap, i) {
        let insert_at = offset * full_size;
        let to_move_size = (arr.count as usize - offset) * full_size;
        arr.group.resize((arr.count as usize + 1) * full_size, 0);
        if to_move_size > 0 {
            arr.group
                .copy_within(insert_at..insert_at + to_move_size, insert_at + full_size);
        }
        arr.count += 1;
        set_position(&mut arr.bitmap, i);
    }

    let start = offset * full_size;
    if !write_usize(&mut arr.group[start..start + USIZE_BYTES], vlen) {
        return 0;
    }
    let data_start = start + USIZE_BYTES;
    arr.group[data_start..data_start + vlen].copy_from_slice(&val[..vlen]);
    1
}

fn sparse_array_group_get<'a>(
    arr: &'a SparseArrayGroup,
    i: u32,
    outsize: Option<&mut usize>,
) -> Option<&'a [u8]> {
    if !is_position_occupied(&arr.bitmap, i) {
        return None;
    }

    let offset = position_to_offset(&arr.bitmap, i);
    let full_size = full_elem_size(arr.elem_size);
    let start = offset.checked_mul(full_size)?;
    let item_size = read_usize(arr.group.get(start..start + USIZE_BYTES)?)?;
    if item_size == 0 {
        return None;
    }

    if let Some(outsize) = outsize {
        *outsize = item_size;
    }

    let data_start = start + USIZE_BYTES;
    arr.group.get(data_start..data_start + item_size)
}

fn rehash_and_grow_table(dict: &mut SparseDict) -> i32 {
    let new_bucket_max = match dict.bucket_max.checked_mul(2) {
        Some(value) if value > 0 => value,
        _ => return 0,
    };

    let mut new_buckets = Vec::with_capacity(new_bucket_max);
    for _ in 0..new_bucket_max {
        new_buckets.push(empty_bucket_slot());
    }

    let mut buckets_rehashed = 0usize;
    for slot in dict.buckets.iter().take(dict.bucket_max) {
        if let Some((hash, key, value)) = parse_bucket_slot(slot) {
            let mut num_probes = 0usize;
            loop {
                let probed_val = quadratic_probe(hash, num_probes, new_bucket_max);
                if bucket_slot_is_empty(&new_buckets[probed_val]) {
                    new_buckets[probed_val] = build_bucket_slot(hash, key, value);
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
    }

    dict.buckets = new_buckets;
    dict.bucket_max = new_bucket_max;
    1
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
    let mut groups = Vec::with_capacity(max_arr_size(maximum));
    for _ in 0..max_arr_size(maximum) {
        groups.push(SparseArrayGroup {
            count: 0,
            elem_size: element_size,
            group: Vec::new(),
            bitmap: [0; BITMAP_SIZE],
        });
    }

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
    if i as usize > arr.maximum {
        return 0;
    }

    let group_index = i as usize / GROUP_SIZE;
    let position = i % GROUP_SIZE as u32;
    let Some(group) = arr.groups.get_mut(group_index) else {
        return 0;
    };

    sparse_array_group_set(group, position, val, vlen)
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
    if i as usize > arr.maximum {
        return None;
    }

    let group_index = i as usize / GROUP_SIZE;
    let position = i % GROUP_SIZE as u32;
    let group = arr.groups.get(group_index)?;
    sparse_array_group_get(group, position, outsize)
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
    let mut buckets = Vec::with_capacity(STARTING_SIZE);
    for _ in 0..STARTING_SIZE {
        buckets.push(empty_bucket_slot());
    }

    Some(Box::new(SparseDict {
        bucket_max: STARTING_SIZE,
        bucket_count: 0,
        buckets,
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
    if klen > key.len() || vlen > value.len() || dict.bucket_max == 0 {
        return 0;
    }

    let key_bytes = &key.as_bytes()[..klen];
    let value_bytes = &value[..vlen];
    let key_hash = hash_fnv1a(key_bytes);
    let mut num_probes = 0usize;

    loop {
        let probed_val = quadratic_probe(key_hash, num_probes, dict.bucket_max);
        let slot = &dict.buckets[probed_val];

        if bucket_slot_is_empty(slot) {
            dict.buckets[probed_val] = build_bucket_slot(key_hash, key_bytes, value_bytes);
            break;
        }

        if let Some((existing_hash, existing_key, _)) = parse_bucket_slot(slot) {
            if existing_hash == key_hash && existing_key.len() == klen && existing_key == key_bytes {
                dict.buckets[probed_val] = build_bucket_slot(key_hash, key_bytes, value_bytes);
                return 1;
            }
        } else {
            return 0;
        }

        num_probes += 1;
        if num_probes > dict.bucket_count {
            return 0;
        }
    }

    dict.bucket_count += 1;
    if (dict.bucket_count as f32) / (dict.bucket_max as f32) >= (RESIZE_PERCENT as f32 / 100.0) {
        return rehash_and_grow_table(dict);
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
    if klen > key.len() || dict.bucket_max == 0 {
        return None;
    }

    let key_bytes = &key.as_bytes()[..klen];
    let key_hash = hash_fnv1a(key_bytes);
    let mut num_probes = 0usize;

    loop {
        let probed_val = quadratic_probe(key_hash, num_probes, dict.bucket_max);
        let slot = dict.buckets.get(probed_val)?;

        if let Some((existing_hash, existing_key, existing_value)) = parse_bucket_slot(slot) {
            if existing_hash == key_hash && existing_key.len() == klen && existing_key == key_bytes {
                if let Some(outsize) = outsize {
                    *outsize = existing_value.len();
                }
                return Some(existing_value);
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
