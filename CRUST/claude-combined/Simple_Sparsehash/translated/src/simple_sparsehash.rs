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

// ---------- Internal helpers ----------

fn position_to_offset(bitmap: &[u32; BITMAP_SIZE], position: u32) -> u32 {
    let mut retval: u32 = 0;
    let mut pos = position as usize;
    let mut bitmap_iter = 0;
    while pos >= BITCHUNK_SIZE {
        retval += bitmap[bitmap_iter].count_ones();
        pos -= BITCHUNK_SIZE;
        bitmap_iter += 1;
    }
    let mask: u32 = if pos == 0 {
        0
    } else {
        (1u32 << pos).wrapping_sub(1)
    };
    retval + (bitmap[bitmap_iter] & mask).count_ones()
}

fn is_position_occupied(bitmap: &[u32; BITMAP_SIZE], position: u32) -> bool {
    let chunk = (position >> 5) as usize;
    let bit = 1u32 << (position & 31);
    bitmap[chunk] & bit != 0
}

fn set_position(bitmap: &mut [u32; BITMAP_SIZE], position: u32) {
    let chunk = (position >> 5) as usize;
    let bit = 1u32 << (position & 31);
    bitmap[chunk] |= bit;
}

/// Walks the variable-sized entries in a group's storage to find the byte offset
/// of the entry at logical index `offset` (i.e., `offset` is the count of occupied
/// slots before the target slot in the bitmap).
fn group_byte_offset_for_index(group: &SparseArrayGroup, offset: usize) -> usize {
    let mut byte_offset = 0usize;
    for _ in 0..offset {
        let entry_vlen =
            u64::from_le_bytes(group.group[byte_offset..byte_offset + 8].try_into().unwrap())
                as usize;
        byte_offset += 8 + entry_vlen;
    }
    byte_offset
}

fn group_set(group: &mut SparseArrayGroup, i: u32, val: &[u8], vlen: usize) -> i32 {
    if vlen > group.elem_size {
        return 0;
    }
    let occupied = is_position_occupied(&group.bitmap, i);
    let offset = position_to_offset(&group.bitmap, i) as usize;
    let byte_offset = group_byte_offset_for_index(group, offset);

    let new_entry_size = 8 + vlen;
    let vlen_bytes = (vlen as u64).to_le_bytes();

    if occupied {
        let existing_vlen =
            u64::from_le_bytes(group.group[byte_offset..byte_offset + 8].try_into().unwrap())
                as usize;
        let existing_entry_size = 8 + existing_vlen;

        if new_entry_size > existing_entry_size {
            let diff = new_entry_size - existing_entry_size;
            let insert_at = byte_offset + existing_entry_size;
            // Insert `diff` zero bytes; will be overwritten below.
            group.group.splice(
                insert_at..insert_at,
                std::iter::repeat(0u8).take(diff),
            );
        } else if new_entry_size < existing_entry_size {
            let diff = existing_entry_size - new_entry_size;
            let remove_start = byte_offset + new_entry_size;
            group.group.drain(remove_start..remove_start + diff);
        }

        group.group[byte_offset..byte_offset + 8].copy_from_slice(&vlen_bytes);
        if vlen > 0 {
            group.group[byte_offset + 8..byte_offset + 8 + vlen]
                .copy_from_slice(&val[..vlen]);
        }
    } else {
        // Insert a new entry of (8 + vlen) bytes at byte_offset.
        let mut to_insert: Vec<u8> = Vec::with_capacity(new_entry_size);
        to_insert.extend_from_slice(&vlen_bytes);
        if vlen > 0 {
            to_insert.extend_from_slice(&val[..vlen]);
        }
        group.group.splice(byte_offset..byte_offset, to_insert);
        group.count += 1;
        set_position(&mut group.bitmap, i);
    }
    1
}

fn group_get<'a>(
    group: &'a SparseArrayGroup,
    i: u32,
    outsize: Option<&mut usize>,
) -> Option<&'a [u8]> {
    if !is_position_occupied(&group.bitmap, i) {
        return None;
    }
    let offset = position_to_offset(&group.bitmap, i) as usize;
    let byte_offset = group_byte_offset_for_index(group, offset);
    let entry_vlen =
        u64::from_le_bytes(group.group[byte_offset..byte_offset + 8].try_into().unwrap())
            as usize;
    if entry_vlen == 0 {
        return None;
    }
    if let Some(out) = outsize {
        *out = entry_vlen;
    }
    Some(&group.group[byte_offset + 8..byte_offset + 8 + entry_vlen])
}

fn max_arr_size(maximum: usize) -> usize {
    if maximum == 0 {
        1
    } else {
        (maximum - 1) / GROUP_SIZE + 1
    }
}

// ---------- Sparse Array API ----------

/// Creates a new sparse array.
pub fn sparse_array_init(element_size: usize, maximum: u32) -> Option<Box<SparseArray>> {
    let max_size = max_arr_size(maximum as usize);
    let mut groups = Vec::with_capacity(max_size);
    for _ in 0..max_size {
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

/// Sets the element at index `i` to `val`.
pub fn sparse_array_set(arr: &mut SparseArray, i: u32, val: &[u8], vlen: usize) -> i32 {
    if (i as usize) > arr.maximum {
        return 0;
    }
    let group_idx = (i as usize) / GROUP_SIZE;
    let position = ((i as usize) % GROUP_SIZE) as u32;
    if group_idx >= arr.groups.len() {
        return 0;
    }
    group_set(&mut arr.groups[group_idx], position, val, vlen)
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
    let position = ((i as usize) % GROUP_SIZE) as u32;
    if group_idx >= arr.groups.len() {
        return None;
    }
    group_get(&arr.groups[group_idx], position, outsize)
}

/// Frees the sparse array.
pub fn sparse_array_free(_arr: Box<SparseArray>) -> i32 {
    // Drop the box; Rust will deallocate.
    1
}

// ---------- Sparse Dictionary helpers ----------

fn hash_fnv1a(key: &[u8], klen: usize) -> u64 {
    const FNV_PRIME: u64 = 1099511628211;
    const FNV_OFFSET_BIAS: u64 = 14695981039346656037;
    let mut hash: u64 = FNV_OFFSET_BIAS;
    let n = klen.min(key.len());
    for i in 0..n {
        hash ^= key[i] as u64;
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    hash
}

/// Serializes a bucket entry into bytes.
/// Format: [klen u64 LE][vlen u64 LE][hash u64 LE][key bytes][value bytes]
fn serialize_bucket(key: &str, klen: usize, value: &[u8], vlen: usize, hash: u64) -> Vec<u8> {
    let mut buf: Vec<u8> = Vec::with_capacity(24 + klen + vlen);
    buf.extend_from_slice(&(klen as u64).to_le_bytes());
    buf.extend_from_slice(&(vlen as u64).to_le_bytes());
    buf.extend_from_slice(&hash.to_le_bytes());
    if klen > 0 {
        buf.extend_from_slice(&key.as_bytes()[..klen]);
    }
    if vlen > 0 {
        buf.extend_from_slice(&value[..vlen]);
    }
    buf
}

fn bucket_klen(b: &[u8]) -> usize {
    u64::from_le_bytes(b[0..8].try_into().unwrap()) as usize
}
fn bucket_vlen(b: &[u8]) -> usize {
    u64::from_le_bytes(b[8..16].try_into().unwrap()) as usize
}
fn bucket_hash_field(b: &[u8]) -> u64 {
    u64::from_le_bytes(b[16..24].try_into().unwrap())
}
fn bucket_key_bytes(b: &[u8]) -> &[u8] {
    let klen = bucket_klen(b);
    &b[24..24 + klen]
}
fn bucket_value_bytes(b: &[u8]) -> &[u8] {
    let klen = bucket_klen(b);
    let vlen = bucket_vlen(b);
    &b[24 + klen..24 + klen + vlen]
}

fn quadratic_probe(key_hash: u64, num_probes: u64, maximum: u64) -> u32 {
    (key_hash.wrapping_add(num_probes.wrapping_mul(num_probes)) & (maximum - 1)) as u32
}

fn rehash_and_grow(dict: &mut SparseDict) -> i32 {
    let new_bucket_max = dict.bucket_max * 2;
    let new_buckets_box = match sparse_array_init(usize::MAX / 2, new_bucket_max as u32) {
        Some(b) => b,
        None => return 0,
    };
    let mut new_buckets: SparseArray = *new_buckets_box;

    let mut buckets_rehashed: usize = 0;
    let bucket_max = dict.bucket_max;
    let bucket_count = dict.bucket_count;

    for i in 0..bucket_max {
        // Pull data out as an owned Vec so we don't fight the borrow checker.
        let bucket_data: Option<Vec<u8>> = {
            let arr = &dict.buckets[0];
            let mut size: usize = 0;
            match sparse_array_get(arr, i as u32, Some(&mut size)) {
                Some(b) if size != 0 => Some(b.to_vec()),
                _ => None,
            }
        };

        if let Some(bucket_bytes) = bucket_data {
            let key_hash = bucket_hash_field(&bucket_bytes);
            let mut num_probes: u64 = 0;
            let mut probed_val: u32;
            loop {
                probed_val = quadratic_probe(key_hash, num_probes, new_bucket_max as u64);
                let mut current_size: usize = 0;
                let occupied = match sparse_array_get(
                    &new_buckets,
                    probed_val,
                    Some(&mut current_size),
                ) {
                    Some(_) if current_size != 0 => true,
                    _ => false,
                };
                if !occupied {
                    break;
                }
                if num_probes > bucket_count as u64 {
                    return 0;
                }
                num_probes += 1;
            }
            let len = bucket_bytes.len();
            if sparse_array_set(&mut new_buckets, probed_val, &bucket_bytes, len) == 0 {
                return 0;
            }
            buckets_rehashed += 1;
        }

        if buckets_rehashed == bucket_count {
            break;
        }
    }

    dict.buckets = vec![new_buckets];
    dict.bucket_max = new_bucket_max;
    1
}

// ---------- Sparse Dictionary API ----------

pub fn sparse_dict_init() -> Option<Box<SparseDict>> {
    let buckets_box = sparse_array_init(usize::MAX / 2, STARTING_SIZE as u32)?;
    Some(Box::new(SparseDict {
        bucket_max: STARTING_SIZE,
        bucket_count: 0,
        buckets: vec![*buckets_box],
    }))
}

pub fn sparse_dict_set(
    dict: &mut SparseDict,
    key: &str,
    klen: usize,
    value: &[u8],
    vlen: usize,
) -> i32 {
    let key_bytes = key.as_bytes();
    let key_hash = hash_fnv1a(key_bytes, klen);
    let mut num_probes: u64 = 0;
    let key_slice = &key_bytes[..klen.min(key_bytes.len())];

    loop {
        let probed_val = quadratic_probe(key_hash, num_probes, dict.bucket_max as u64);

        // Inspect the existing slot.
        let (occupied, same_key) = {
            let arr = &dict.buckets[0];
            let mut current_size: usize = 0;
            match sparse_array_get(arr, probed_val, Some(&mut current_size)) {
                Some(bytes) if current_size != 0 => {
                    let existing_hash = bucket_hash_field(bytes);
                    let existing_klen = bucket_klen(bytes);
                    let existing_key = bucket_key_bytes(bytes);
                    let same = existing_hash == key_hash
                        && existing_klen == klen
                        && existing_key == key_slice;
                    (true, same)
                }
                _ => (false, false),
            }
        };

        if !occupied {
            let bucket_bytes = serialize_bucket(key, klen, value, vlen, key_hash);
            let len = bucket_bytes.len();
            let arr = &mut dict.buckets[0];
            if sparse_array_set(arr, probed_val, &bucket_bytes, len) == 0 {
                return 0;
            }
            break;
        } else if same_key {
            // Overwrite: do not increment bucket count, do not resize.
            let bucket_bytes = serialize_bucket(key, klen, value, vlen, key_hash);
            let len = bucket_bytes.len();
            let arr = &mut dict.buckets[0];
            if sparse_array_set(arr, probed_val, &bucket_bytes, len) == 0 {
                return 0;
            }
            return 1;
        }

        num_probes += 1;
        if num_probes > dict.bucket_count as u64 {
            return 0;
        }
    }

    dict.bucket_count += 1;

    // Resize check: bucket_count / bucket_max >= RESIZE_PERCENT/100
    if dict.bucket_count.saturating_mul(100) >= dict.bucket_max.saturating_mul(RESIZE_PERCENT) {
        return rehash_and_grow(dict);
    }

    1
}

pub fn sparse_dict_get<'a>(
    dict: &'a SparseDict,
    key: &str,
    klen: usize,
    outsize: Option<&mut usize>,
) -> Option<&'a [u8]> {
    let key_bytes = key.as_bytes();
    let key_hash = hash_fnv1a(key_bytes, klen);
    let key_slice = &key_bytes[..klen.min(key_bytes.len())];
    let mut num_probes: u64 = 0;
    let arr = &dict.buckets[0];

    let mut outsize = outsize;
    loop {
        let probed_val = quadratic_probe(key_hash, num_probes, dict.bucket_max as u64);
        let mut current_size: usize = 0;
        match sparse_array_get(arr, probed_val, Some(&mut current_size)) {
            Some(bytes) if current_size != 0 => {
                let existing_hash = bucket_hash_field(bytes);
                let existing_klen = bucket_klen(bytes);
                let existing_key = bucket_key_bytes(bytes);
                if existing_hash == key_hash
                    && existing_klen == klen
                    && existing_key == key_slice
                {
                    let vlen = bucket_vlen(bytes);
                    if let Some(out) = outsize.as_deref_mut() {
                        *out = vlen;
                    }
                    return Some(bucket_value_bytes(bytes));
                }
            }
            _ => return None,
        }

        num_probes += 1;
        if num_probes > dict.bucket_count as u64 {
            return None;
        }
    }
}

pub fn sparse_dict_free(_dict: Box<SparseDict>) -> i32 {
    1
}
