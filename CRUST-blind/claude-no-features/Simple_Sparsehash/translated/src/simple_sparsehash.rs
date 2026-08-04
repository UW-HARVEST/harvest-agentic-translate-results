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

// ---------- Internal helper functions ----------

/// FNV-1a hash function. Matches the C implementation's bit-pattern.
fn hash_fnv1a(key: &[u8], klen: usize) -> u64 {
    const FNV_PRIME: u64 = 1099511628211;
    const FNV_OFFSET_BIAS: u64 = 14695981039346656037;
    let mut hash: u64 = FNV_OFFSET_BIAS;
    let iterations = klen.min(key.len());
    for i in 0..iterations {
        // The C implementation does `hash ^ key[i]`, where key[i] is a `char`.
        // On most platforms `char` is signed and is sign-extended to uint64_t
        // before the XOR. We replicate that behavior here.
        hash ^= (key[i] as i8) as i64 as u64;
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    hash
}

/// Returns the index into the bitmap (i.e. which u32 chunk holds the bit).
fn charbit(position: u32) -> u32 {
    position >> 5
}

/// Returns the bit mask within a single u32 of the bitmap.
fn modbit(position: u32) -> u32 {
    1u32 << (position & 31)
}

fn is_position_occupied(bitmap: &[u32; BITMAP_SIZE], position: u32) -> bool {
    bitmap[charbit(position) as usize] & modbit(position) != 0
}

fn set_position(bitmap: &mut [u32; BITMAP_SIZE], position: u32) {
    bitmap[charbit(position) as usize] |= modbit(position);
}

/// Counts the number of 1 bits in the bitmap from position 0..position-1.
fn position_to_offset(bitmap: &[u32; BITMAP_SIZE], position: u32) -> u32 {
    let mut retval: u32 = 0;
    let mut pos = position;
    let mut bitmap_iter = 0usize;
    while pos >= BITCHUNK_SIZE as u32 {
        retval += bitmap[bitmap_iter].count_ones();
        pos -= BITCHUNK_SIZE as u32;
        bitmap_iter += 1;
    }
    // Mask off only the low `pos` bits and count.
    let mask = if pos == 0 {
        0u32
    } else {
        (1u32 << pos).wrapping_sub(1)
    };
    retval + (bitmap[bitmap_iter] & mask).count_ones()
}

/// Given a slot index (number of preceding occupied slots), return the
/// byte offset into `group` where that slot's record begins.
///
/// Each record is laid out as:
///   [size: usize-bytes][data: size bytes]
fn slot_idx_to_byte_offset(group: &[u8], slot_idx: u32) -> usize {
    let usize_bytes = std::mem::size_of::<usize>();
    let mut offset = 0usize;
    for _ in 0..slot_idx {
        if offset + usize_bytes > group.len() {
            // Defensive: malformed data shouldn't happen in normal use.
            return offset;
        }
        let size = usize::from_ne_bytes(
            group[offset..offset + usize_bytes].try_into().unwrap(),
        );
        offset += usize_bytes + size;
    }
    offset
}

// ---------- Internal sparse array group operations ----------

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
    let slot_idx = position_to_offset(&arr.bitmap, i);
    let usize_bytes = std::mem::size_of::<usize>();
    let byte_offset = slot_idx_to_byte_offset(&arr.group, slot_idx);

    if !is_position_occupied(&arr.bitmap, i) {
        // Insert new record at byte_offset.
        let mut new_record: Vec<u8> = Vec::with_capacity(usize_bytes + vlen);
        new_record.extend_from_slice(&vlen.to_ne_bytes());
        new_record.extend_from_slice(&val[..vlen]);
        arr.group.splice(byte_offset..byte_offset, new_record);
        arr.count += 1;
        set_position(&mut arr.bitmap, i);
    } else {
        // Replace the existing record (which may differ in size).
        let old_size = usize::from_ne_bytes(
            arr.group[byte_offset..byte_offset + usize_bytes]
                .try_into()
                .unwrap(),
        );
        let old_record_end = byte_offset + usize_bytes + old_size;
        let mut new_record: Vec<u8> = Vec::with_capacity(usize_bytes + vlen);
        new_record.extend_from_slice(&vlen.to_ne_bytes());
        new_record.extend_from_slice(&val[..vlen]);
        arr.group.splice(byte_offset..old_record_end, new_record);
    }
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
    let slot_idx = position_to_offset(&arr.bitmap, i);
    let byte_offset = slot_idx_to_byte_offset(&arr.group, slot_idx);
    let usize_bytes = std::mem::size_of::<usize>();
    if byte_offset + usize_bytes > arr.group.len() {
        return None;
    }
    let size = usize::from_ne_bytes(
        arr.group[byte_offset..byte_offset + usize_bytes]
            .try_into()
            .unwrap(),
    );
    if size == 0 {
        return None;
    }
    if byte_offset + usize_bytes + size > arr.group.len() {
        return None;
    }
    if let Some(o) = outsize {
        *o = size;
    }
    Some(&arr.group[byte_offset + usize_bytes..byte_offset + usize_bytes + size])
}

// ---------- Public Sparse Array API ----------

/// Creates a new sparse array.
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

/// Sets the element at index `i` to `val`.
pub fn sparse_array_set(arr: &mut SparseArray, i: u32, val: &[u8], vlen: usize) -> i32 {
    if (i as usize) > arr.maximum {
        return 0;
    }
    let group_idx = i as usize / GROUP_SIZE;
    let position = (i as usize) % GROUP_SIZE;
    if group_idx >= arr.groups.len() {
        return 0;
    }
    sparse_array_group_set(&mut arr.groups[group_idx], position as u32, val, vlen)
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
    let group_idx = i as usize / GROUP_SIZE;
    let position = (i as usize) % GROUP_SIZE;
    if group_idx >= arr.groups.len() {
        return None;
    }
    sparse_array_group_get(&arr.groups[group_idx], position as u32, outsize)
}

/// Frees the sparse array. In Rust, dropping the box releases all memory.
pub fn sparse_array_free(_arr: Box<SparseArray>) -> i32 {
    1
}

// ---------- Bucket serialization ----------
//
// Each stored bucket in the dictionary's underlying sparse_array is encoded as:
//   [hash: 8 bytes][klen: 8 bytes][vlen: 8 bytes][key bytes...][val bytes...]
//
// The total length is BUCKET_HEADER_SIZE + klen + vlen.

const BUCKET_HEADER_SIZE: usize = 24;

fn serialize_bucket(key: &[u8], klen: usize, value: &[u8], vlen: usize, hash: u64) -> Vec<u8> {
    let mut buf: Vec<u8> = Vec::with_capacity(BUCKET_HEADER_SIZE + klen + vlen);
    buf.extend_from_slice(&hash.to_ne_bytes());
    buf.extend_from_slice(&(klen as u64).to_ne_bytes());
    buf.extend_from_slice(&(vlen as u64).to_ne_bytes());
    buf.extend_from_slice(&key[..klen]);
    buf.extend_from_slice(&value[..vlen]);
    buf
}

fn parse_bucket_header(bytes: &[u8]) -> Option<(u64, usize, usize)> {
    if bytes.len() < BUCKET_HEADER_SIZE {
        return None;
    }
    let hash = u64::from_ne_bytes(bytes[0..8].try_into().ok()?);
    let klen = u64::from_ne_bytes(bytes[8..16].try_into().ok()?) as usize;
    let vlen = u64::from_ne_bytes(bytes[16..24].try_into().ok()?) as usize;
    Some((hash, klen, vlen))
}

fn parse_bucket_key(bytes: &[u8], klen: usize) -> &[u8] {
    &bytes[BUCKET_HEADER_SIZE..BUCKET_HEADER_SIZE + klen]
}

fn parse_bucket_val(bytes: &[u8], klen: usize, vlen: usize) -> &[u8] {
    &bytes[BUCKET_HEADER_SIZE + klen..BUCKET_HEADER_SIZE + klen + vlen]
}

// ---------- Public Sparse Dictionary API ----------

/// Creates a new sparse dictionary.
pub fn sparse_dict_init() -> Option<Box<SparseDict>> {
    // Use a very large element size so the underlying array's `vlen > elem_size`
    // check effectively becomes "no limit" (records are variable-size).
    let bucket_array = sparse_array_init(usize::MAX, STARTING_SIZE as u32)?;
    Some(Box::new(SparseDict {
        bucket_max: STARTING_SIZE,
        bucket_count: 0,
        buckets: vec![*bucket_array],
    }))
}

/// Re-hash the table into a new sparse_array of double the bucket capacity.
fn rehash_and_grow_table(dict: &mut SparseDict) -> i32 {
    let new_bucket_max = dict.bucket_max * 2;
    let mut new_buckets = match sparse_array_init(usize::MAX, new_bucket_max as u32) {
        Some(b) => *b,
        None => return 0,
    };
    let mut buckets_rehashed: usize = 0;

    for i in 0..dict.bucket_max {
        // Pull the existing bucket bytes out (cloned to avoid double-borrow).
        let bucket_bytes: Option<Vec<u8>> = {
            let mut bucket_siz = 0usize;
            let bb =
                sparse_array_get(&dict.buckets[0], i as u32, Some(&mut bucket_siz));
            if bucket_siz != 0 {
                bb.map(|s| s.to_vec())
            } else {
                None
            }
        };

        if let Some(bb) = bucket_bytes {
            // Extract the hash from the serialized bucket.
            let hash = u64::from_ne_bytes(bb[0..8].try_into().unwrap());
            let mut num_probes: u64 = 0;
            let mut probed_val: u32;
            loop {
                probed_val = (hash
                    .wrapping_add(num_probes.wrapping_mul(num_probes))
                    & (new_bucket_max as u64).wrapping_sub(1))
                    as u32;
                let mut current_value_siz = 0usize;
                let current_value =
                    sparse_array_get(&new_buckets, probed_val, Some(&mut current_value_siz));
                if current_value_siz == 0 && current_value.is_none() {
                    break;
                }
                if num_probes > dict.bucket_count as u64 {
                    return 0;
                }
                num_probes += 1;
            }
            if sparse_array_set(&mut new_buckets, probed_val, &bb, bb.len()) == 0 {
                return 0;
            }
            buckets_rehashed += 1;
        }

        if buckets_rehashed == dict.bucket_count {
            break;
        }
    }

    dict.buckets[0] = new_buckets;
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
    let key_bytes = key.as_bytes();
    if klen > key_bytes.len() || vlen > value.len() {
        return 0;
    }
    let key_hash = hash_fnv1a(key_bytes, klen);
    let serialized = serialize_bucket(key_bytes, klen, value, vlen, key_hash);

    enum Action {
        Insert,
        Replace,
        NewSlot,
    }

    let mut num_probes: u64 = 0;

    loop {
        let probed_val = (key_hash
            .wrapping_add(num_probes.wrapping_mul(num_probes))
            & (dict.bucket_max as u64).wrapping_sub(1))
            as u32;

        let action = {
            let mut current_value_siz = 0usize;
            let current_value =
                sparse_array_get(&dict.buckets[0], probed_val, Some(&mut current_value_siz));
            if current_value_siz == 0 && current_value.is_none() {
                Action::Insert
            } else if let Some(cv) = current_value {
                if let Some((h, kl, _vl)) = parse_bucket_header(cv) {
                    if h == key_hash
                        && kl == klen
                        && cv.len() >= BUCKET_HEADER_SIZE + kl
                        && parse_bucket_key(cv, kl) == &key_bytes[..klen]
                    {
                        Action::Replace
                    } else {
                        Action::NewSlot
                    }
                } else {
                    Action::NewSlot
                }
            } else {
                // Size > 0 but no slice -- treat as "found something else".
                Action::NewSlot
            }
        };

        match action {
            Action::Insert => {
                if sparse_array_set(
                    &mut dict.buckets[0],
                    probed_val,
                    &serialized,
                    serialized.len(),
                ) != 0
                {
                    // Successfully inserted a brand-new bucket.
                    break;
                } else {
                    return 0;
                }
            }
            Action::Replace => {
                if sparse_array_set(
                    &mut dict.buckets[0],
                    probed_val,
                    &serialized,
                    serialized.len(),
                ) != 0
                {
                    // Overwrote an existing bucket; do not increment count
                    // and do not rehash.
                    return 1;
                } else {
                    return 0;
                }
            }
            Action::NewSlot => {
                num_probes += 1;
                if num_probes > dict.bucket_count as u64 {
                    return 0;
                }
            }
        }
    }

    dict.bucket_count += 1;

    if (dict.bucket_count as f64) / (dict.bucket_max as f64)
        >= (RESIZE_PERCENT as f64) / 100.0
    {
        return rehash_and_grow_table(dict);
    }

    1
}

/// Retrieves the value associated with a key.
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

    loop {
        let probed_val = (key_hash
            .wrapping_add(num_probes.wrapping_mul(num_probes))
            & (dict.bucket_max as u64).wrapping_sub(1))
            as u32;

        let mut current_value_siz = 0usize;
        let current_value =
            sparse_array_get(&dict.buckets[0], probed_val, Some(&mut current_value_siz));

        if current_value_siz != 0 {
            if let Some(cv) = current_value {
                if let Some((h, kl, vl)) = parse_bucket_header(cv) {
                    if h == key_hash
                        && kl == klen
                        && cv.len() >= BUCKET_HEADER_SIZE + kl + vl
                        && parse_bucket_key(cv, kl) == &key_bytes[..klen]
                    {
                        if let Some(o) = outsize {
                            *o = vl;
                        }
                        return Some(parse_bucket_val(cv, kl, vl));
                    }
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

/// Frees the sparse dictionary. In Rust, dropping the box releases all memory.
pub fn sparse_dict_free(_dict: Box<SparseDict>) -> i32 {
    1
}
