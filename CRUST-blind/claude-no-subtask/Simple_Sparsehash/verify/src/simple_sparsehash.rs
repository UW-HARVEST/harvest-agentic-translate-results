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

/// Element size used by the dictionary's underlying sparse array. Each
/// bucket entry is stored as a serialized blob of (hash, klen, vlen, key,
/// value), so this constant must be large enough to fit the encoded form
/// of any inserted key/value pair.
const DICT_BUCKET_ELEM_SIZE: usize = 256;

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

/// FNV-1a hash function. Matches the C implementation for ASCII keys.
fn hash_fnv1a(key: &[u8]) -> u64 {
    const FNV_PRIME: u64 = 1099511628211;
    const FNV_OFFSET_BIAS: u64 = 14695981039346656037;
    let mut hash: u64 = FNV_OFFSET_BIAS;
    for &b in key {
        hash ^= b as u64;
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    hash
}

#[inline]
fn charbit(position: u32) -> usize {
    (position >> 5) as usize
}

#[inline]
fn modbit(position: u32) -> u32 {
    1u32 << (position & 31)
}

#[inline]
fn popcount_32(x: u32) -> u32 {
    x.count_ones()
}

/// Maps a "position" (logical index) to "offset" (slot index in compact storage).
fn position_to_offset(bitmap: &[u32; BITMAP_SIZE], position: u32) -> u32 {
    let mut retval: u32 = 0;
    let mut pos = position;
    let mut bitmap_iter: usize = 0;

    while pos >= BITCHUNK_SIZE as u32 {
        retval += popcount_32(bitmap[bitmap_iter]);
        bitmap_iter += 1;
        pos -= BITCHUNK_SIZE as u32;
    }

    // pos is in [0, 31] now, so (1 << pos) is safe.
    let mask = if pos == 0 { 0 } else { (1u32 << pos) - 1 };
    retval + popcount_32(bitmap[bitmap_iter] & mask)
}

#[inline]
fn is_position_occupied(bitmap: &[u32; BITMAP_SIZE], position: u32) -> bool {
    bitmap[charbit(position)] & modbit(position) != 0
}

#[inline]
fn set_position(bitmap: &mut [u32; BITMAP_SIZE], position: u32) {
    bitmap[charbit(position)] |= modbit(position);
}

/// Quadratic probing function. `maximum` must be a power of two.
#[inline]
fn quadratic_probe(key_hash: u64, num_probes: u64, maximum: usize) -> u32 {
    let probed =
        key_hash.wrapping_add(num_probes.wrapping_mul(num_probes)) & (maximum as u64 - 1);
    probed as u32
}

/// Encode a bucket entry into a byte vector.
fn encode_bucket(hash: u64, key: &[u8], val: &[u8]) -> Vec<u8> {
    let klen = key.len();
    let vlen = val.len();
    let mut buf = Vec::with_capacity(24 + klen + vlen);
    buf.extend_from_slice(&hash.to_ne_bytes());
    buf.extend_from_slice(&klen.to_ne_bytes());
    buf.extend_from_slice(&vlen.to_ne_bytes());
    buf.extend_from_slice(key);
    buf.extend_from_slice(val);
    buf
}

/// Decode a bucket entry from a byte slice.
/// Returns (hash, key_bytes, val_bytes).
fn decode_bucket(data: &[u8]) -> Option<(u64, &[u8], &[u8])> {
    if data.len() < 24 {
        return None;
    }
    let hash = u64::from_ne_bytes(data[0..8].try_into().ok()?);
    let klen = usize::from_ne_bytes(data[8..16].try_into().ok()?);
    let vlen = usize::from_ne_bytes(data[16..24].try_into().ok()?);
    if data.len() < 24 + klen + vlen {
        return None;
    }
    let key = &data[24..24 + klen];
    let val = &data[24 + klen..24 + klen + vlen];
    Some((hash, key, val))
}

// ---------- Internal sparse_array_group functions ----------

fn sparse_array_group_set(
    arr: &mut SparseArrayGroup,
    i: u32,
    val: &[u8],
    vlen: usize,
) -> i32 {
    if vlen > arr.elem_size {
        return 0;
    }
    if vlen > val.len() {
        return 0;
    }

    let usize_sz = std::mem::size_of::<usize>();
    let full_elem_size = arr.elem_size + usize_sz;
    let offset = position_to_offset(&arr.bitmap, i) as usize;

    if !is_position_occupied(&arr.bitmap, i) {
        let to_move_siz = (arr.count as usize - offset) * full_elem_size;
        let new_total = (arr.count as usize + 1) * full_elem_size;
        arr.group.resize(new_total, 0);

        if to_move_siz > 0 {
            let src_start = offset * full_elem_size;
            let dst_start = (offset + 1) * full_elem_size;
            arr.group
                .copy_within(src_start..src_start + to_move_siz, dst_start);
        }

        arr.count += 1;
        set_position(&mut arr.bitmap, i);
    }

    let dest_start = offset * full_elem_size;
    let size_bytes = vlen.to_ne_bytes();
    arr.group[dest_start..dest_start + usize_sz].copy_from_slice(&size_bytes);
    arr.group[dest_start + usize_sz..dest_start + usize_sz + vlen]
        .copy_from_slice(&val[..vlen]);

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

    let usize_sz = std::mem::size_of::<usize>();
    let full_elem_size = arr.elem_size + usize_sz;
    let offset = position_to_offset(&arr.bitmap, i) as usize;
    let item_start = offset * full_elem_size;

    if item_start + usize_sz > arr.group.len() {
        return None;
    }
    let size_bytes: [u8; 8] = arr.group[item_start..item_start + usize_sz].try_into().ok()?;
    let item_size = usize::from_ne_bytes(size_bytes);

    // Per the C implementation: zero-sized items are treated as missing.
    if item_size == 0 {
        return None;
    }

    let value_start = item_start + usize_sz;
    if value_start + item_size > arr.group.len() {
        return None;
    }

    if let Some(o) = outsize {
        *o = item_size;
    }

    Some(&arr.group[value_start..value_start + item_size])
}

// ---------- Public Sparse Array API ----------

/// Creates a new sparse array.
pub fn sparse_array_init(element_size: usize, maximum: u32) -> Option<Box<SparseArray>> {
    let max_arr_size = if maximum == 0 {
        1
    } else {
        (maximum as usize - 1) / GROUP_SIZE + 1
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

/// Sets the element at index `i` to `val`.
pub fn sparse_array_set(arr: &mut SparseArray, i: u32, val: &[u8], vlen: usize) -> i32 {
    if (i as usize) > arr.maximum {
        return 0;
    }
    let group_idx = (i as usize) / GROUP_SIZE;
    if group_idx >= arr.groups.len() {
        return 0;
    }
    let position = i % (GROUP_SIZE as u32);
    let group = &mut arr.groups[group_idx];
    sparse_array_group_set(group, position, val, vlen)
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
    if group_idx >= arr.groups.len() {
        return None;
    }
    let position = i % (GROUP_SIZE as u32);
    let group = &arr.groups[group_idx];
    sparse_array_group_get(group, position, outsize)
}

/// Frees the sparse array. With Rust's Box, dropping the Box releases
/// owned memory automatically.
pub fn sparse_array_free(_arr: Box<SparseArray>) -> i32 {
    1
}

// ---------- Sparse Dictionary API ----------

/// Creates a new sparse dictionary.
pub fn sparse_dict_init() -> Option<Box<SparseDict>> {
    let bucket_array = sparse_array_init(DICT_BUCKET_ELEM_SIZE, STARTING_SIZE as u32)?;
    Some(Box::new(SparseDict {
        bucket_max: STARTING_SIZE,
        bucket_count: 0,
        buckets: vec![*bucket_array],
    }))
}

/// Internal: re-insert a single encoded bucket into a fresh SparseArray
/// using quadratic probing.
fn rehash_insert(
    new_array: &mut SparseArray,
    new_bucket_max: usize,
    bucket_data: &[u8],
    key_hash: u64,
    safety_max_probes: usize,
) -> i32 {
    let mut num_probes: u64 = 0;
    loop {
        let probed = quadratic_probe(key_hash, num_probes, new_bucket_max);
        let mut cur_siz: usize = 0;
        let cur = sparse_array_get(new_array, probed, Some(&mut cur_siz));
        if cur.is_none() || cur_siz == 0 {
            return sparse_array_set(new_array, probed, bucket_data, bucket_data.len());
        }
        if num_probes as usize > safety_max_probes {
            return 0;
        }
        num_probes += 1;
    }
}

/// Internal: rehash and grow the dictionary's underlying bucket array.
fn rehash_and_grow(dict: &mut SparseDict) -> i32 {
    let new_bucket_max = dict.bucket_max * 2;
    let new_bucket_array_box = match sparse_array_init(DICT_BUCKET_ELEM_SIZE, new_bucket_max as u32)
    {
        Some(b) => b,
        None => return 0,
    };
    let mut new_array: SparseArray = *new_bucket_array_box;

    let mut buckets_rehashed: usize = 0;
    let total_to_rehash = dict.bucket_count;
    let old_bucket_max = dict.bucket_max;

    for i in 0..old_bucket_max {
        let mut bucket_siz: usize = 0;
        let bucket_data: Option<Vec<u8>> = {
            let arr = &dict.buckets[0];
            sparse_array_get(arr, i as u32, Some(&mut bucket_siz)).map(|d| d.to_vec())
        };

        if let Some(data) = bucket_data {
            if bucket_siz != 0 {
                let key_hash = match decode_bucket(&data) {
                    Some((h, _, _)) => h,
                    None => continue,
                };
                if rehash_insert(
                    &mut new_array,
                    new_bucket_max,
                    &data,
                    key_hash,
                    total_to_rehash,
                ) == 0
                {
                    return 0;
                }
                buckets_rehashed += 1;
            }
        }

        if buckets_rehashed == total_to_rehash {
            break;
        }
    }

    dict.buckets = vec![new_array];
    dict.bucket_max = new_bucket_max;
    1
}

/// Inserts a key/value pair into the dictionary.
pub fn sparse_dict_set(
    dict: &mut SparseDict,
    key: &str,
    klen: usize,
    value: &[u8],
    vlen: usize,
) -> i32 {
    let kbytes = key.as_bytes();
    if klen > kbytes.len() {
        return 0;
    }
    if vlen > value.len() {
        return 0;
    }
    let key_bytes = &kbytes[..klen];
    let val_bytes = &value[..vlen];
    let key_hash = hash_fnv1a(key_bytes);

    enum Action {
        Insert(u32),
        Replace(u32),
        Fail,
    }

    let action: Action = {
        let bucket_array = &dict.buckets[0];
        let bucket_max = dict.bucket_max;
        let bucket_count = dict.bucket_count;
        let mut num_probes: u64 = 0;

        loop {
            let probed = quadratic_probe(key_hash, num_probes, bucket_max);
            let mut current_size: usize = 0;
            let current_data = sparse_array_get(bucket_array, probed, Some(&mut current_size));

            if current_data.is_none() || current_size == 0 {
                break Action::Insert(probed);
            }

            // Slot occupied: see if it's the same key (we'd then replace).
            if let Some((existing_hash, existing_key, _)) = decode_bucket(current_data.unwrap()) {
                if existing_hash == key_hash
                    && existing_key.len() == klen
                    && existing_key == key_bytes
                {
                    break Action::Replace(probed);
                }
            }

            num_probes += 1;
            if num_probes > bucket_count as u64 {
                break Action::Fail;
            }
        }
    };

    match action {
        Action::Fail => 0,
        Action::Replace(slot) => {
            let bucket_data = encode_bucket(key_hash, key_bytes, val_bytes);
            let len = bucket_data.len();
            let bucket_array = &mut dict.buckets[0];
            if sparse_array_set(bucket_array, slot, &bucket_data, len) == 0 {
                return 0;
            }
            1
        }
        Action::Insert(slot) => {
            let bucket_data = encode_bucket(key_hash, key_bytes, val_bytes);
            let len = bucket_data.len();
            {
                let bucket_array = &mut dict.buckets[0];
                if sparse_array_set(bucket_array, slot, &bucket_data, len) == 0 {
                    return 0;
                }
            }
            dict.bucket_count += 1;

            if dict.bucket_count * 100 >= dict.bucket_max * RESIZE_PERCENT {
                return rehash_and_grow(dict);
            }
            1
        }
    }
}

/// Retrieves the value associated with a key.
pub fn sparse_dict_get<'a>(
    dict: &'a SparseDict,
    key: &str,
    klen: usize,
    mut outsize: Option<&mut usize>,
) -> Option<&'a [u8]> {
    let kbytes = key.as_bytes();
    if klen > kbytes.len() {
        return None;
    }
    let key_bytes = &kbytes[..klen];
    let key_hash = hash_fnv1a(key_bytes);
    let bucket_array = &dict.buckets[0];
    let mut num_probes: u64 = 0;

    loop {
        let probed = quadratic_probe(key_hash, num_probes, dict.bucket_max);
        let mut current_size: usize = 0;
        let current_data = sparse_array_get(bucket_array, probed, Some(&mut current_size));

        if let Some(data) = current_data {
            if current_size == 0 {
                return None;
            }
            if let Some((existing_hash, existing_key, existing_val)) = decode_bucket(data) {
                if existing_hash == key_hash
                    && existing_key.len() == klen
                    && existing_key == key_bytes
                {
                    if let Some(o) = outsize.as_mut() {
                        **o = existing_val.len();
                    }
                    return Some(existing_val);
                }
            }
        } else {
            return None;
        }

        num_probes += 1;
        if num_probes > dict.bucket_count as u64 {
            return None;
        }
    }
}

/// Frees the sparse dictionary.
pub fn sparse_dict_free(_dict: Box<SparseDict>) -> i32 {
    1
}
