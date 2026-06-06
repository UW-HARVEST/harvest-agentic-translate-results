use std::convert::TryInto;

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

// ----- Internal helpers -----

/// FNV-1a hash function (matches the C implementation).
fn hash_fnv1a(key: &str, klen: usize) -> u64 {
    const FNV_PRIME: u64 = 1099511628211;
    const FNV_OFFSET_BIAS: u64 = 14695981039346656037;
    let bytes = key.as_bytes();
    let iterations = klen.min(bytes.len());
    let mut hash: u64 = FNV_OFFSET_BIAS;
    for i in 0..iterations {
        hash ^= bytes[i] as u64;
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    hash
}

#[inline]
fn charbit(position: u32) -> u32 {
    position >> 5
}

#[inline]
fn modbit(position: u32) -> u32 {
    1u32 << (position & 31)
}

/// Counts the number of 1-bits in the bitmap from positions 0 to position-1.
fn position_to_offset(bitmap: &[u32; BITMAP_SIZE], position: u32) -> u32 {
    let mut retval: u32 = 0;
    let mut pos = position;
    let mut iter = 0usize;
    while pos >= BITCHUNK_SIZE as u32 {
        retval += bitmap[iter].count_ones();
        iter += 1;
        pos -= BITCHUNK_SIZE as u32;
    }
    let mask = (1u32 << pos).wrapping_sub(1);
    retval + (bitmap[iter] & mask).count_ones()
}

#[inline]
fn is_position_occupied(bitmap: &[u32; BITMAP_SIZE], position: u32) -> bool {
    bitmap[charbit(position) as usize] & modbit(position) != 0
}

#[inline]
fn set_position(bitmap: &mut [u32; BITMAP_SIZE], position: u32) {
    bitmap[charbit(position) as usize] |= modbit(position);
}

/// Each occupied slot in `group.group` is stored as:
///   8 bytes (u64 LE): size of the data
///   `size` bytes: the data itself
///
/// This function returns the byte position in `group` of the slot at array
/// `offset` (0-indexed among the occupied slots).
fn find_byte_offset(group: &[u8], offset: u32) -> usize {
    let mut byte_pos: usize = 0;
    for _ in 0..offset {
        let size = u64::from_le_bytes(
            group[byte_pos..byte_pos + 8].try_into().unwrap(),
        ) as usize;
        byte_pos += 8 + size;
    }
    byte_pos
}

#[inline]
fn read_size_at(group: &[u8], byte_pos: usize) -> usize {
    u64::from_le_bytes(group[byte_pos..byte_pos + 8].try_into().unwrap()) as usize
}

// ----- Sparse Array API -----

/// Creates a new sparse array.
pub fn sparse_array_init(element_size: usize, maximum: u32) -> Option<Box<SparseArray>> {
    let max_arr_size = if maximum == 0 {
        1
    } else {
        ((maximum as usize - 1) / GROUP_SIZE) + 1
    };
    let mut groups = Vec::with_capacity(max_arr_size);
    for _ in 0..max_arr_size {
        groups.push(SparseArrayGroup {
            count: 0,
            elem_size: element_size,
            group: Vec::new(),
            bitmap: [0u32; BITMAP_SIZE],
        });
    }
    Some(Box::new(SparseArray {
        maximum: maximum as usize,
        groups,
    }))
}

/// Sets the element at index `i` to `val[..vlen]`.
pub fn sparse_array_set(arr: &mut SparseArray, i: u32, val: &[u8], vlen: usize) -> i32 {
    if (i as usize) > arr.maximum {
        return 0;
    }
    let group_idx = (i as usize) / GROUP_SIZE;
    let position = (i as usize % GROUP_SIZE) as u32;
    let group = &mut arr.groups[group_idx];

    if vlen > group.elem_size {
        return 0;
    }
    if vlen > val.len() {
        return 0;
    }

    let offset = position_to_offset(&group.bitmap, position);
    let byte_pos = find_byte_offset(&group.group, offset);

    if is_position_occupied(&group.bitmap, position) {
        let old_size = read_size_at(&group.group, byte_pos);
        let mut replacement: Vec<u8> = Vec::with_capacity(8 + vlen);
        replacement.extend_from_slice(&(vlen as u64).to_le_bytes());
        replacement.extend_from_slice(&val[..vlen]);
        group
            .group
            .splice(byte_pos..byte_pos + 8 + old_size, replacement);
    } else {
        let mut insertion: Vec<u8> = Vec::with_capacity(8 + vlen);
        insertion.extend_from_slice(&(vlen as u64).to_le_bytes());
        insertion.extend_from_slice(&val[..vlen]);
        group.group.splice(byte_pos..byte_pos, insertion);
        group.count += 1;
        set_position(&mut group.bitmap, position);
    }
    1
}

/// Retrieves the element at index `i`.
pub fn sparse_array_get<'a>(
    arr: &'a SparseArray,
    i: u32,
    outsize: Option<&mut usize>,
) -> Option<&'a [u8]> {
    if (i as usize) > arr.maximum {
        return None;
    }
    let group_idx = (i as usize) / GROUP_SIZE;
    let position = (i as usize % GROUP_SIZE) as u32;
    let group = &arr.groups[group_idx];
    if !is_position_occupied(&group.bitmap, position) {
        return None;
    }
    let offset = position_to_offset(&group.bitmap, position);
    let byte_pos = find_byte_offset(&group.group, offset);
    let size = read_size_at(&group.group, byte_pos);
    if size == 0 {
        return None;
    }
    if let Some(out) = outsize {
        *out = size;
    }
    Some(&group.group[byte_pos + 8..byte_pos + 8 + size])
}

/// Frees the sparse array. (No-op besides dropping the box.)
pub fn sparse_array_free(_arr: Box<SparseArray>) -> i32 {
    1
}

// ----- Sparse Dictionary helpers -----

/// Element size used internally for the dictionary's underlying array.
/// Since stored buckets have variable length (key + value bytes plus a small
/// header), we use a generous upper bound that disables the per-element size
/// check while not producing any allocation overhead in our variable-length
/// storage scheme.
const DICT_ELEM_SIZE: usize = usize::MAX;

/// Serialized bucket layout:
///   [klen: 8 bytes LE]
///   [vlen: 8 bytes LE]
///   [hash: 8 bytes LE]
///   [key bytes: klen bytes]
///   [val bytes: vlen bytes]
fn serialize_bucket(key: &str, klen: usize, value: &[u8], vlen: usize, hash: u64) -> Vec<u8> {
    let mut out = Vec::with_capacity(24 + klen + vlen);
    out.extend_from_slice(&(klen as u64).to_le_bytes());
    out.extend_from_slice(&(vlen as u64).to_le_bytes());
    out.extend_from_slice(&hash.to_le_bytes());
    out.extend_from_slice(&key.as_bytes()[..klen]);
    out.extend_from_slice(&value[..vlen]);
    out
}

#[inline]
fn bucket_klen(data: &[u8]) -> usize {
    u64::from_le_bytes(data[0..8].try_into().unwrap()) as usize
}

#[inline]
fn bucket_vlen(data: &[u8]) -> usize {
    u64::from_le_bytes(data[8..16].try_into().unwrap()) as usize
}

#[inline]
fn bucket_hash(data: &[u8]) -> u64 {
    u64::from_le_bytes(data[16..24].try_into().unwrap())
}

#[inline]
fn bucket_key<'a>(data: &'a [u8], klen: usize) -> &'a [u8] {
    &data[24..24 + klen]
}

#[inline]
fn quadratic_probe(key_hash: u64, num_probes: u64, maximum: usize) -> u32 {
    let m = maximum as u64;
    (key_hash.wrapping_add(num_probes.wrapping_mul(num_probes)) & (m - 1)) as u32
}

fn rehash_and_grow(dict: &mut SparseDict) -> i32 {
    let new_bucket_max = dict.bucket_max * 2;
    let mut new_arr_box = match sparse_array_init(DICT_ELEM_SIZE, new_bucket_max as u32) {
        Some(b) => b,
        None => return 0,
    };

    let mut buckets_rehashed: usize = 0;
    let old_bucket_max = dict.bucket_max;
    let old_bucket_count = dict.bucket_count;

    for i in 0..old_bucket_max as u32 {
        // Snapshot the bucket data, so we can release the immutable borrow.
        let bucket_data: Option<Vec<u8>> = {
            let arr = &dict.buckets[0];
            sparse_array_get(arr, i, None).map(|s| s.to_vec())
        };

        if let Some(data) = bucket_data {
            let key_hash = bucket_hash(&data);
            let mut num_probes: u64 = 0;
            let probed_val;
            loop {
                let pv = quadratic_probe(key_hash, num_probes, new_bucket_max);
                let occupied = sparse_array_get(&new_arr_box, pv, None).is_some();
                if !occupied {
                    probed_val = pv;
                    break;
                }
                if num_probes > old_bucket_count as u64 {
                    return 0;
                }
                num_probes += 1;
            }
            let data_len = data.len();
            if sparse_array_set(&mut new_arr_box, probed_val, &data, data_len) == 0 {
                return 0;
            }
            buckets_rehashed += 1;
            if buckets_rehashed == old_bucket_count {
                break;
            }
        }
    }

    dict.buckets[0] = *new_arr_box;
    dict.bucket_max = new_bucket_max;
    1
}

// ----- Sparse Dictionary API -----

/// Creates a new sparse dictionary.
pub fn sparse_dict_init() -> Option<Box<SparseDict>> {
    let arr_box = sparse_array_init(DICT_ELEM_SIZE, STARTING_SIZE as u32)?;
    Some(Box::new(SparseDict {
        bucket_max: STARTING_SIZE,
        bucket_count: 0,
        buckets: vec![*arr_box],
    }))
}

/// Inserts a key/value pair into the dictionary.
pub fn sparse_dict_set(
    dict: &mut SparseDict,
    key: &str,
    klen: usize,
    value: &[u8],
    vlen: usize,
) -> i32 {
    let key_hash = hash_fnv1a(key, klen);
    let key_bytes = key.as_bytes();
    if klen > key_bytes.len() || vlen > value.len() {
        return 0;
    }

    let mut num_probes: u64 = 0;

    loop {
        let probed_val = quadratic_probe(key_hash, num_probes, dict.bucket_max);

        // First, examine the slot. If it matches our key, replace; if empty,
        // insert; otherwise probe more. We must release the immutable borrow
        // before performing a mutable set.
        enum Action {
            Empty,
            SameKey,
            Different,
        }
        let action: Action = {
            let arr = &dict.buckets[0];
            match sparse_array_get(arr, probed_val, None) {
                None => Action::Empty,
                Some(data) => {
                    let stored_klen = bucket_klen(data);
                    let stored_hash = bucket_hash(data);
                    if stored_hash == key_hash
                        && stored_klen == klen
                        && bucket_key(data, stored_klen) == &key_bytes[..klen]
                    {
                        Action::SameKey
                    } else {
                        Action::Different
                    }
                }
            }
        };

        match action {
            Action::Empty => {
                let serialized = serialize_bucket(key, klen, value, vlen, key_hash);
                let arr = &mut dict.buckets[0];
                let len = serialized.len();
                if sparse_array_set(arr, probed_val, &serialized, len) == 0 {
                    return 0;
                }
                break;
            }
            Action::SameKey => {
                let serialized = serialize_bucket(key, klen, value, vlen, key_hash);
                let arr = &mut dict.buckets[0];
                let len = serialized.len();
                if sparse_array_set(arr, probed_val, &serialized, len) == 0 {
                    return 0;
                }
                return 1;
            }
            Action::Different => {
                num_probes += 1;
                if num_probes > dict.bucket_count as u64 {
                    return 0;
                }
            }
        }
    }

    dict.bucket_count += 1;
    if dict.bucket_count.saturating_mul(100) >= dict.bucket_max.saturating_mul(RESIZE_PERCENT) {
        return rehash_and_grow(dict);
    }
    1
}

/// Retrieves the value associated with `key`.
pub fn sparse_dict_get<'a>(
    dict: &'a SparseDict,
    key: &str,
    klen: usize,
    outsize: Option<&mut usize>,
) -> Option<&'a [u8]> {
    let key_hash = hash_fnv1a(key, klen);
    let key_bytes = key.as_bytes();
    if klen > key_bytes.len() {
        return None;
    }

    let arr = &dict.buckets[0];
    let mut num_probes: u64 = 0;
    loop {
        let probed_val = quadratic_probe(key_hash, num_probes, dict.bucket_max);
        match sparse_array_get(arr, probed_val, None) {
            Some(data) => {
                let stored_klen = bucket_klen(data);
                let stored_vlen = bucket_vlen(data);
                let stored_hash = bucket_hash(data);
                if stored_hash == key_hash
                    && stored_klen == klen
                    && bucket_key(data, stored_klen) == &key_bytes[..klen]
                {
                    if let Some(out) = outsize {
                        *out = stored_vlen;
                    }
                    let val_start = 24 + stored_klen;
                    return Some(&data[val_start..val_start + stored_vlen]);
                }
            }
            None => return None,
        }
        num_probes += 1;
        if num_probes > dict.bucket_count as u64 {
            return None;
        }
    }
}

/// Frees the sparse dictionary. (No-op besides dropping the box.)
pub fn sparse_dict_free(_dict: Box<SparseDict>) -> i32 {
    1
}
