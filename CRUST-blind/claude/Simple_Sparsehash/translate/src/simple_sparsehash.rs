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

/// FNV-1a 64-bit hash.
fn hash_fnv1a(key: &[u8], klen: usize) -> u64 {
    const FNV_PRIME: u64 = 1099511628211u64;
    const FNV_OFFSET_BIAS: u64 = 14695981039346656037u64;
    let mut hash: u64 = FNV_OFFSET_BIAS;
    let n = klen.min(key.len());
    for i in 0..n {
        hash ^= key[i] as u64;
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

#[inline]
fn is_position_occupied(bitmap: &[u32; BITMAP_SIZE], position: u32) -> bool {
    (bitmap[charbit(position) as usize] & modbit(position)) != 0
}

#[inline]
fn set_position(bitmap: &mut [u32; BITMAP_SIZE], position: u32) {
    bitmap[charbit(position) as usize] |= modbit(position);
}

/// Counts the number of occupied positions in the bitmap before `position`.
fn position_to_offset(bitmap: &[u32; BITMAP_SIZE], position: u32) -> u32 {
    let mut retval: u32 = 0;
    let mut pos = position;
    let mut iter: usize = 0;
    while pos >= BITCHUNK_SIZE as u32 {
        retval += bitmap[iter].count_ones();
        pos -= BITCHUNK_SIZE as u32;
        iter += 1;
    }
    let mask: u32 = if pos == 0 { 0 } else { (1u32 << pos).wrapping_sub(1) };
    retval + (bitmap[iter] & mask).count_ones()
}

/// Returns the byte offset in `group_buf` where the record at logical `offset` lives.
fn record_byte_offset(group_buf: &[u8], offset: u32) -> usize {
    let mut byte: usize = 0;
    for _ in 0..offset {
        let mut sz_bytes = [0u8; 8];
        sz_bytes.copy_from_slice(&group_buf[byte..byte + 8]);
        let size = usize::from_le_bytes(sz_bytes);
        byte += 8 + size;
    }
    byte
}

/// Reads the size header at byte offset.
fn read_size_at(group_buf: &[u8], byte_offset: usize) -> usize {
    let mut sz_bytes = [0u8; 8];
    sz_bytes.copy_from_slice(&group_buf[byte_offset..byte_offset + 8]);
    usize::from_le_bytes(sz_bytes)
}

fn _sparse_array_group_set(
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

    let offset = position_to_offset(&arr.bitmap, i);
    let byte_offset = record_byte_offset(&arr.group, offset);

    // Build the new record bytes: [size: 8 bytes][data: vlen bytes]
    let mut new_record: Vec<u8> = Vec::with_capacity(8 + vlen);
    new_record.extend_from_slice(&vlen.to_le_bytes());
    new_record.extend_from_slice(&val[..vlen]);

    if !is_position_occupied(&arr.bitmap, i) {
        // Insert new record at byte_offset.
        arr.group.splice(byte_offset..byte_offset, new_record);
        arr.count += 1;
        set_position(&mut arr.bitmap, i);
    } else {
        // Replace existing record.
        let existing_size = read_size_at(&arr.group, byte_offset);
        let existing_total = 8 + existing_size;
        arr.group
            .splice(byte_offset..byte_offset + existing_total, new_record);
    }
    1
}

fn _sparse_array_group_get<'a>(
    arr: &'a SparseArrayGroup,
    i: u32,
    outsize: Option<&mut usize>,
) -> Option<&'a [u8]> {
    if !is_position_occupied(&arr.bitmap, i) {
        return None;
    }
    let offset = position_to_offset(&arr.bitmap, i);
    let byte_offset = record_byte_offset(&arr.group, offset);
    let size = read_size_at(&arr.group, byte_offset);
    if size == 0 {
        return None;
    }
    if let Some(out) = outsize {
        *out = size;
    }
    Some(&arr.group[byte_offset + 8..byte_offset + 8 + size])
}

// ---------- Sparse Array API ----------

pub fn sparse_array_init(element_size: usize, maximum: u32) -> Option<Box<SparseArray>> {
    let max_arr_size = if maximum == 0 {
        1
    } else {
        (maximum as usize - 1) / GROUP_SIZE + 1
    };
    let mut groups: Vec<SparseArrayGroup> = Vec::with_capacity(max_arr_size);
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

pub fn sparse_array_set(arr: &mut SparseArray, i: u32, val: &[u8], vlen: usize) -> i32 {
    if (i as usize) > arr.maximum {
        return 0;
    }
    let group_idx = (i as usize) / GROUP_SIZE;
    let position = i % (GROUP_SIZE as u32);
    if group_idx >= arr.groups.len() {
        return 0;
    }
    _sparse_array_group_set(&mut arr.groups[group_idx], position, val, vlen)
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
    let position = i % (GROUP_SIZE as u32);
    if group_idx >= arr.groups.len() {
        return None;
    }
    _sparse_array_group_get(&arr.groups[group_idx], position, outsize)
}

pub fn sparse_array_free(_arr: Box<SparseArray>) -> i32 {
    // Box drops automatically when going out of scope.
    1
}

// ---------- Sparse Dictionary helpers ----------

#[inline]
fn quadratic_probe(key_hash: u64, num_probes: u64, maximum: u64) -> u64 {
    key_hash.wrapping_add(num_probes.wrapping_mul(num_probes)) & (maximum - 1)
}

/// Serialize a bucket into bytes:
/// [klen: 8 bytes LE][vlen: 8 bytes LE][hash: 8 bytes LE][key bytes][value bytes]
fn serialize_bucket(klen: usize, vlen: usize, hash: u64, key: &[u8], value: &[u8]) -> Vec<u8> {
    let mut buf = Vec::with_capacity(24 + klen + vlen);
    buf.extend_from_slice(&klen.to_le_bytes());
    buf.extend_from_slice(&vlen.to_le_bytes());
    buf.extend_from_slice(&hash.to_le_bytes());
    buf.extend_from_slice(&key[..klen]);
    buf.extend_from_slice(&value[..vlen]);
    buf
}

fn deserialize_bucket_header(bytes: &[u8]) -> (usize, usize, u64) {
    let mut a = [0u8; 8];
    let mut b = [0u8; 8];
    let mut c = [0u8; 8];
    a.copy_from_slice(&bytes[0..8]);
    b.copy_from_slice(&bytes[8..16]);
    c.copy_from_slice(&bytes[16..24]);
    (
        usize::from_le_bytes(a),
        usize::from_le_bytes(b),
        u64::from_le_bytes(c),
    )
}

fn bucket_key_slice<'a>(bytes: &'a [u8], klen: usize) -> &'a [u8] {
    &bytes[24..24 + klen]
}

fn bucket_value_slice<'a>(bytes: &'a [u8], klen: usize, vlen: usize) -> &'a [u8] {
    &bytes[24 + klen..24 + klen + vlen]
}

/// A safely large element size for the dict's underlying sparse_array. We use
/// variable-length records inside the group, so this just needs to be large
/// enough that `vlen <= elem_size` is always true for serialized buckets.
const DICT_ELEM_SIZE: usize = usize::MAX / 4;

fn rehash_and_grow_table(dict: &mut SparseDict) -> i32 {
    let new_bucket_max = dict.bucket_max * 2;
    let mut new_arr = match sparse_array_init(DICT_ELEM_SIZE, new_bucket_max as u32) {
        Some(b) => *b,
        None => return 0,
    };

    // Collect all serialized bucket bytes from the old array.
    let mut entries: Vec<Vec<u8>> = Vec::with_capacity(dict.bucket_count);
    {
        let old_arr = &dict.buckets[0];
        for i in 0..dict.bucket_max as u32 {
            let mut size: usize = 0;
            if let Some(bytes) = sparse_array_get(old_arr, i, Some(&mut size)) {
                if size != 0 {
                    entries.push(bytes.to_vec());
                    if entries.len() == dict.bucket_count {
                        break;
                    }
                }
            }
        }
    }

    // Re-insert each entry into the new array using quadratic probing.
    for entry in entries.iter() {
        let (_klen, _vlen, key_hash) = deserialize_bucket_header(entry);
        let mut num_probes: u64 = 0;
        loop {
            let probed_val =
                quadratic_probe(key_hash, num_probes, new_bucket_max as u64) as u32;
            let mut size: usize = 0;
            let is_empty = sparse_array_get(&new_arr, probed_val, Some(&mut size)).is_none();
            if is_empty {
                if sparse_array_set(&mut new_arr, probed_val, entry, entry.len()) == 0 {
                    return 0;
                }
                break;
            }
            num_probes += 1;
            if num_probes > dict.bucket_count as u64 {
                return 0;
            }
        }
    }

    dict.buckets[0] = new_arr;
    dict.bucket_max = new_bucket_max;
    1
}

// ---------- Sparse Dictionary API ----------

pub fn sparse_dict_init() -> Option<Box<SparseDict>> {
    let arr = sparse_array_init(DICT_ELEM_SIZE, STARTING_SIZE as u32)?;
    let mut buckets: Vec<SparseArray> = Vec::with_capacity(1);
    buckets.push(*arr);
    Some(Box::new(SparseDict {
        bucket_max: STARTING_SIZE,
        bucket_count: 0,
        buckets,
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
    if klen > key_bytes.len() {
        return 0;
    }
    if vlen > value.len() {
        return 0;
    }
    let key_hash = hash_fnv1a(key_bytes, klen);
    let mut num_probes: u64 = 0;

    loop {
        let probed_val =
            quadratic_probe(key_hash, num_probes, dict.bucket_max as u64) as u32;

        // Inspect current slot; capture decision in a small local enum so
        // the borrow ends before we mutate.
        enum Decision {
            Empty,
            SameKey,
            DifferentKey,
        }
        let decision = {
            let arr = &dict.buckets[0];
            let mut size: usize = 0;
            match sparse_array_get(arr, probed_val, Some(&mut size)) {
                None => Decision::Empty,
                Some(bytes) => {
                    let (existing_klen, _existing_vlen, existing_hash) =
                        deserialize_bucket_header(bytes);
                    if existing_hash == key_hash
                        && existing_klen == klen
                        && bucket_key_slice(bytes, existing_klen) == &key_bytes[..klen]
                    {
                        Decision::SameKey
                    } else {
                        Decision::DifferentKey
                    }
                }
            }
        };

        match decision {
            Decision::Empty => {
                let serialized = serialize_bucket(klen, vlen, key_hash, key_bytes, value);
                let arr_mut = &mut dict.buckets[0];
                let n = serialized.len();
                if sparse_array_set(arr_mut, probed_val, &serialized, n) == 0 {
                    return 0;
                }
                break;
            }
            Decision::SameKey => {
                let serialized = serialize_bucket(klen, vlen, key_hash, key_bytes, value);
                let arr_mut = &mut dict.buckets[0];
                let n = serialized.len();
                if sparse_array_set(arr_mut, probed_val, &serialized, n) == 0 {
                    return 0;
                }
                // Overwrite, no count change, no resize.
                return 1;
            }
            Decision::DifferentKey => {
                num_probes += 1;
                if num_probes > dict.bucket_count as u64 {
                    return 0;
                }
            }
        }
    }

    dict.bucket_count += 1;

    // Resize if load factor reaches RESIZE_PERCENT/100.
    if dict.bucket_count.saturating_mul(100) >= dict.bucket_max.saturating_mul(RESIZE_PERCENT) {
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
    let key_bytes = key.as_bytes();
    if klen > key_bytes.len() {
        return None;
    }
    let key_hash = hash_fnv1a(key_bytes, klen);
    let mut num_probes: u64 = 0;
    let mut outsize = outsize;

    loop {
        let probed_val =
            quadratic_probe(key_hash, num_probes, dict.bucket_max as u64) as u32;
        let arr = &dict.buckets[0];
        let mut size: usize = 0;
        match sparse_array_get(arr, probed_val, Some(&mut size)) {
            Some(bytes) => {
                let (existing_klen, existing_vlen, existing_hash) =
                    deserialize_bucket_header(bytes);
                if existing_hash == key_hash
                    && existing_klen == klen
                    && bucket_key_slice(bytes, existing_klen) == &key_bytes[..klen]
                {
                    if let Some(out) = outsize.as_mut() {
                        **out = existing_vlen;
                    }
                    return Some(bucket_value_slice(bytes, existing_klen, existing_vlen));
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
    // Box drops automatically.
    1
}
