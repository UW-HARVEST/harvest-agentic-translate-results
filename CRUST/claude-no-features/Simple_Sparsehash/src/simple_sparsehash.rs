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

// ---------- Constants & Helpers ----------

/// Size of the serialized bucket header: hash (u64) + klen (u64) + vlen (u64) = 24 bytes.
const DICT_BUCKET_HEADER: usize = 24;
/// elem_size used for the SparseArray that backs a SparseDict.
/// Must be large enough to hold the serialized bucket: header + key + value.
const DICT_ELEM_SIZE: usize = 64;

#[inline]
fn full_elem_size(elem_size: usize) -> usize {
    elem_size + std::mem::size_of::<usize>()
}

#[inline]
fn max_arr_size(maximum: usize) -> usize {
    if maximum == 0 {
        1
    } else {
        (maximum - 1) / GROUP_SIZE + 1
    }
}

/// FNV-1a hash, matching the C implementation (for klen < 256).
fn hash_fnv1a(key: &[u8]) -> u64 {
    const FNV_PRIME: u64 = 1099511628211u64;
    const FNV_OFFSET_BIAS: u64 = 14695981039346656037u64;
    let mut hash = FNV_OFFSET_BIAS;
    for &b in key {
        hash ^= b as u64;
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    hash
}

#[inline]
fn modbit(position: u32) -> u32 {
    1u32 << (position & 31)
}

#[inline]
fn charbit(position: u32) -> usize {
    (position >> 5) as usize
}

#[inline]
fn is_position_occupied(bitmap: &[u32], position: u32) -> bool {
    (bitmap[charbit(position)] & modbit(position)) != 0
}

#[inline]
fn set_position(bitmap: &mut [u32], position: u32) {
    bitmap[charbit(position)] |= modbit(position);
}

fn position_to_offset(bitmap: &[u32; BITMAP_SIZE], position: u32) -> u32 {
    let mut retval: u32 = 0;
    let mut pos = position;
    let mut bitmap_iter: usize = 0;
    while pos >= BITCHUNK_SIZE as u32 {
        retval += bitmap[bitmap_iter].count_ones();
        bitmap_iter += 1;
        pos -= BITCHUNK_SIZE as u32;
    }
    // pos is now in [0, 31], so (1u32 << pos) is well-defined.
    retval + (bitmap[bitmap_iter] & ((1u32 << pos).wrapping_sub(1))).count_ones()
}

// ---------- Serialization for buckets in dict ----------

fn serialize_bucket(hash: u64, key: &str, klen: usize, value: &[u8], vlen: usize) -> Vec<u8> {
    let mut buf = Vec::with_capacity(DICT_BUCKET_HEADER + klen + vlen);
    buf.extend_from_slice(&hash.to_ne_bytes());
    buf.extend_from_slice(&(klen as u64).to_ne_bytes());
    buf.extend_from_slice(&(vlen as u64).to_ne_bytes());
    buf.extend_from_slice(&key.as_bytes()[..klen]);
    buf.extend_from_slice(&value[..vlen]);
    buf
}

fn read_u64_at(bytes: &[u8], offset: usize) -> u64 {
    let mut a = [0u8; 8];
    a.copy_from_slice(&bytes[offset..offset + 8]);
    u64::from_ne_bytes(a)
}

fn bucket_hash(bytes: &[u8]) -> u64 {
    read_u64_at(bytes, 0)
}

fn bucket_klen(bytes: &[u8]) -> usize {
    read_u64_at(bytes, 8) as usize
}

fn bucket_vlen(bytes: &[u8]) -> usize {
    read_u64_at(bytes, 16) as usize
}

fn bucket_key_bytes(bytes: &[u8]) -> &[u8] {
    let klen = bucket_klen(bytes);
    &bytes[DICT_BUCKET_HEADER..DICT_BUCKET_HEADER + klen]
}

#[allow(dead_code)]
fn bucket_value_bytes(bytes: &[u8]) -> &[u8] {
    let klen = bucket_klen(bytes);
    let vlen = bucket_vlen(bytes);
    &bytes[DICT_BUCKET_HEADER + klen..DICT_BUCKET_HEADER + klen + vlen]
}

// ---------- Sparse Array API ----------

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

pub fn sparse_array_set(arr: &mut SparseArray, i: u32, val: &[u8], vlen: usize) -> i32 {
    if (i as usize) > arr.maximum {
        return 0;
    }
    let group_idx = (i as usize) / GROUP_SIZE;
    let position = (i as usize) % GROUP_SIZE;
    if group_idx >= arr.groups.len() {
        return 0;
    }
    let group = &mut arr.groups[group_idx];

    if vlen > group.elem_size {
        return 0;
    }
    if val.len() < vlen {
        return 0;
    }

    let fes = full_elem_size(group.elem_size);
    let pos32 = position as u32;
    let offset = position_to_offset(&group.bitmap, pos32) as usize;

    if !is_position_occupied(&group.bitmap, pos32) {
        let to_move_size = (group.count as usize - offset) * fes;
        let new_size = (group.count as usize + 1) * fes;
        group.group.resize(new_size, 0);
        if to_move_size > 0 {
            let src_start = offset * fes;
            let dst_start = (offset + 1) * fes;
            group.group.copy_within(src_start..src_start + to_move_size, dst_start);
        }
        group.count += 1;
        set_position(&mut group.bitmap, pos32);
    }

    // Write the size, then the data.
    let dest_start = offset * fes;
    let size_bytes = vlen.to_ne_bytes();
    let size_len = size_bytes.len();
    group.group[dest_start..dest_start + size_len].copy_from_slice(&size_bytes);
    let data_start = dest_start + size_len;
    group.group[data_start..data_start + vlen].copy_from_slice(&val[..vlen]);

    1
}

pub fn sparse_array_get<'a>(
    arr: &'a SparseArray,
    i: u32,
    outsize: Option<&mut usize>,
) -> Option<&'a [u8]> {
    if (i as usize) > arr.maximum {
        return None;
    }
    let group_idx = (i as usize) / GROUP_SIZE;
    let position = (i as usize) % GROUP_SIZE;
    if group_idx >= arr.groups.len() {
        return None;
    }
    let group = &arr.groups[group_idx];
    let pos32 = position as u32;

    if !is_position_occupied(&group.bitmap, pos32) {
        return None;
    }

    let fes = full_elem_size(group.elem_size);
    let offset = position_to_offset(&group.bitmap, pos32) as usize;
    let dest_start = offset * fes;
    let size_len = std::mem::size_of::<usize>();

    let mut size_arr = [0u8; std::mem::size_of::<usize>()];
    size_arr.copy_from_slice(&group.group[dest_start..dest_start + size_len]);
    let item_size = usize::from_ne_bytes(size_arr);

    if item_size == 0 {
        return None;
    }

    if let Some(out) = outsize {
        *out = item_size;
    }

    let data_start = dest_start + size_len;
    Some(&group.group[data_start..data_start + item_size])
}

pub fn sparse_array_free(_arr: Box<SparseArray>) -> i32 {
    // Box is dropped automatically.
    1
}

// ---------- Sparse Dictionary API ----------

pub fn sparse_dict_init() -> Option<Box<SparseDict>> {
    let arr = sparse_array_init(DICT_ELEM_SIZE, STARTING_SIZE as u32)?;
    Some(Box::new(SparseDict {
        bucket_max: STARTING_SIZE,
        bucket_count: 0,
        buckets: vec![*arr],
    }))
}

fn rehash_and_grow_table(dict: &mut SparseDict) -> i32 {
    let new_bucket_max = dict.bucket_max * 2;
    let elem_size = dict.buckets[0]
        .groups
        .first()
        .map(|g| g.elem_size)
        .unwrap_or(DICT_ELEM_SIZE);

    let mut new_buckets = match sparse_array_init(elem_size, new_bucket_max as u32) {
        Some(b) => b,
        None => return 0,
    };

    let bucket_count = dict.bucket_count;
    let mut buckets_rehashed: usize = 0;
    let old_bucket_max = dict.bucket_max;

    for i in 0..old_bucket_max {
        // Get a copy of the serialized bucket bytes from the OLD array.
        let bucket_copy: Option<Vec<u8>> = sparse_array_get(&dict.buckets[0], i as u32, None)
            .map(|s| s.to_vec());

        if let Some(bucket_bytes) = bucket_copy {
            let key_hash = bucket_hash(&bucket_bytes);
            let mut num_probes: u64 = 0;
            let probed_val: u32;
            loop {
                let np_sq = num_probes.wrapping_mul(num_probes);
                let candidate = (key_hash.wrapping_add(np_sq)
                    & ((new_bucket_max - 1) as u64)) as u32;
                let occupied = sparse_array_get(&new_buckets, candidate, None).is_some();
                if !occupied {
                    probed_val = candidate;
                    break;
                }
                if num_probes > bucket_count as u64 {
                    return 0;
                }
                num_probes += 1;
            }

            if sparse_array_set(&mut new_buckets, probed_val, &bucket_bytes, bucket_bytes.len())
                == 0
            {
                return 0;
            }
            buckets_rehashed += 1;
        }

        if buckets_rehashed == bucket_count {
            break;
        }
    }

    dict.buckets[0] = *new_buckets;
    dict.bucket_max = new_bucket_max;

    1
}

pub fn sparse_dict_set(
    dict: &mut SparseDict,
    key: &str,
    klen: usize,
    value: &[u8],
    vlen: usize,
) -> i32 {
    if klen > key.as_bytes().len() || vlen > value.len() {
        return 0;
    }
    let key_bytes = &key.as_bytes()[..klen];
    let key_hash = hash_fnv1a(key_bytes);
    let serialized = serialize_bucket(key_hash, key, klen, value, vlen);

    let mut num_probes: u64 = 0;
    let probed_val: u32;
    let mut overwriting = false;

    loop {
        let np_sq = num_probes.wrapping_mul(num_probes);
        let candidate =
            (key_hash.wrapping_add(np_sq) & ((dict.bucket_max - 1) as u64)) as u32;

        // Check existing slot.
        let result = {
            let arr = &dict.buckets[0];
            match sparse_array_get(arr, candidate, None) {
                None => None,
                Some(b) => {
                    let h = bucket_hash(b);
                    let kl = bucket_klen(b);
                    if h == key_hash && kl == klen && bucket_key_bytes(b) == key_bytes {
                        Some(true) // found same key
                    } else {
                        Some(false) // occupied by different key
                    }
                }
            }
        };

        match result {
            None => {
                probed_val = candidate;
                break;
            }
            Some(true) => {
                probed_val = candidate;
                overwriting = true;
                break;
            }
            Some(false) => {}
        }

        num_probes += 1;
        if num_probes > dict.bucket_count as u64 {
            // Hash table is full / pathological state.
            return 0;
        }
    }

    let arr_mut = &mut dict.buckets[0];
    if sparse_array_set(arr_mut, probed_val, &serialized, serialized.len()) == 0 {
        return 0;
    }

    if overwriting {
        return 1;
    }

    dict.bucket_count += 1;

    // Resize check: if bucket_count / bucket_max >= RESIZE_PERCENT/100, rehash.
    if dict.bucket_count * 100 >= dict.bucket_max * RESIZE_PERCENT {
        return rehash_and_grow_table(dict);
    }

    1
}

pub fn sparse_dict_get<'a>(
    dict: &'a SparseDict,
    key: &str,
    klen: usize,
    outsize: Option<&mut usize>,
) -> Option<&'a [u8]> {
    if klen > key.as_bytes().len() {
        return None;
    }
    let key_bytes = &key.as_bytes()[..klen];
    let key_hash = hash_fnv1a(key_bytes);
    let mut num_probes: u64 = 0;

    loop {
        let np_sq = num_probes.wrapping_mul(num_probes);
        let candidate =
            (key_hash.wrapping_add(np_sq) & ((dict.bucket_max - 1) as u64)) as u32;

        let arr = &dict.buckets[0];
        match sparse_array_get(arr, candidate, None) {
            Some(bucket_bytes) => {
                let h = bucket_hash(bucket_bytes);
                let kl = bucket_klen(bucket_bytes);
                if h == key_hash && kl == klen && bucket_key_bytes(bucket_bytes) == key_bytes {
                    let vlen = bucket_vlen(bucket_bytes);
                    if let Some(out) = outsize {
                        *out = vlen;
                    }
                    let start = DICT_BUCKET_HEADER + kl;
                    return Some(&bucket_bytes[start..start + vlen]);
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

pub fn sparse_dict_free(_dict: Box<SparseDict>) -> i32 {
    // Box is dropped automatically.
    1
}
