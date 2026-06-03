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

/// FNV-1a hash function.
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

/// Returns position >> 5 (which u32 chunk holds the bit for this position).
fn charbit(position: u32) -> usize {
    (position >> 5) as usize
}

/// Returns 1 << (position & 31) (the bit mask within a u32 chunk).
fn modbit(position: u32) -> u32 {
    1u32 << (position & 31)
}

/// Population count of bits in `x`.
fn popcount_32(x: u32) -> u32 {
    x.count_ones()
}

/// Maps a logical position to the offset (number of occupied slots before it).
fn position_to_offset(bitmap: &[u32; BITMAP_SIZE], position: u32) -> u32 {
    let mut retval: u32 = 0;
    let mut pos = position;
    let mut bitmap_iter: usize = 0;
    while pos >= BITCHUNK_SIZE as u32 {
        retval += popcount_32(bitmap[bitmap_iter]);
        bitmap_iter += 1;
        pos -= BITCHUNK_SIZE as u32;
    }
    // pos is now in [0, 31]. (1 << pos) - 1 fits in u32 when pos < 32.
    let mask: u32 = if pos == 0 { 0 } else { (1u32 << pos).wrapping_sub(1) };
    retval + popcount_32(bitmap[bitmap_iter] & mask)
}

/// Returns true if a slot is occupied.
fn is_position_occupied(bitmap: &[u32; BITMAP_SIZE], position: u32) -> bool {
    (bitmap[charbit(position)] & modbit(position)) != 0
}

/// Sets a position bit in the bitmap.
fn set_position(bitmap: &mut [u32; BITMAP_SIZE], position: u32) {
    bitmap[charbit(position)] |= modbit(position);
}

/// Number of groups required to hold `maximum` elements.
fn max_arr_size(maximum: usize) -> usize {
    if maximum == 0 {
        0
    } else {
        (maximum - 1) / GROUP_SIZE + 1
    }
}

// ---------- Sparse array group operations (private) ----------

fn _sparse_array_group_set(
    group: &mut SparseArrayGroup,
    i: u32,
    val: &[u8],
    vlen: usize,
) -> i32 {
    if vlen > group.elem_size {
        return 0;
    }
    let full_size = group.elem_size + std::mem::size_of::<usize>();
    let offset = position_to_offset(&group.bitmap, i) as usize;

    if !is_position_occupied(&group.bitmap, i) {
        let to_move_size = (group.count as usize - offset) * full_size;
        // Grow buffer to hold one more element.
        let new_total = (group.count as usize + 1) * full_size;
        group.group.resize(new_total, 0);

        if to_move_size > 0 {
            let src_start = offset * full_size;
            let src_end = src_start + to_move_size;
            let dst_start = (offset + 1) * full_size;
            group.group.copy_within(src_start..src_end, dst_start);
        }

        group.count += 1;
        set_position(&mut group.bitmap, i);
    }

    let dest_start = offset * full_size;
    let size_bytes = vlen.to_ne_bytes();
    let header_len = size_bytes.len();
    group.group[dest_start..dest_start + header_len].copy_from_slice(&size_bytes);
    let val_start = dest_start + header_len;
    let copy_len = vlen.min(val.len());
    group.group[val_start..val_start + copy_len].copy_from_slice(&val[..copy_len]);
    // Zero any remainder of the element slot beyond the copied data, up to elem_size.
    if copy_len < group.elem_size {
        for b in &mut group.group[val_start + copy_len..val_start + group.elem_size] {
            *b = 0;
        }
    }

    1
}

fn _sparse_array_group_get<'a>(
    group: &'a SparseArrayGroup,
    i: u32,
    outsize: Option<&mut usize>,
) -> Option<&'a [u8]> {
    if !is_position_occupied(&group.bitmap, i) {
        return None;
    }
    let full_size = group.elem_size + std::mem::size_of::<usize>();
    let offset = position_to_offset(&group.bitmap, i) as usize;
    let item_start = offset * full_size;
    let header_len = std::mem::size_of::<usize>();
    if item_start + header_len > group.group.len() {
        return None;
    }
    let size_bytes: [u8; std::mem::size_of::<usize>()] =
        group.group[item_start..item_start + header_len].try_into().ok()?;
    let item_size = usize::from_ne_bytes(size_bytes);
    if item_size == 0 {
        return None;
    }
    if let Some(out) = outsize {
        *out = item_size;
    }
    let val_start = item_start + header_len;
    if val_start + item_size > group.group.len() {
        return None;
    }
    Some(&group.group[val_start..val_start + item_size])
}

// ---------- Sparse Array API ----------

/// Creates a new sparse array.
pub fn sparse_array_init(element_size: usize, maximum: u32) -> Option<Box<SparseArray>> {
    let max_arr = max_arr_size(maximum as usize);
    let mut groups: Vec<SparseArrayGroup> = Vec::with_capacity(max_arr);
    for _ in 0..max_arr {
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
    _sparse_array_group_set(&mut arr.groups[group_idx], position, val, vlen)
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
    _sparse_array_group_get(&arr.groups[group_idx], position, outsize)
}

/// Frees the sparse array.
pub fn sparse_array_free(_arr: Box<SparseArray>) -> i32 {
    // Box drops automatically.
    1
}

// ---------- Sparse Dictionary helpers ----------
// We treat `dict.buckets` as a Vec of `bucket_max` slots, where each slot is
// a SparseArray. An empty slot has `maximum == 0` and no groups. A filled
// slot has a single group containing the serialized bucket.
//
// Serialized bucket format (in group.group):
//   [hash:u64 (8 bytes, native-endian)]
//   [klen:usize (size_of::<usize>() bytes)]
//   [vlen:usize (size_of::<usize>() bytes)]
//   [key bytes (klen)]
//   [value bytes (vlen)]

const HASH_BYTES: usize = 8;
const USIZE_BYTES: usize = std::mem::size_of::<usize>();
const HEADER_BYTES: usize = HASH_BYTES + USIZE_BYTES + USIZE_BYTES;

fn empty_slot() -> SparseArray {
    SparseArray {
        maximum: 0,
        groups: Vec::new(),
    }
}

fn make_filled_slot(serialized: Vec<u8>) -> SparseArray {
    let mut bitmap = [0u32; BITMAP_SIZE];
    bitmap[0] = 1; // position 0 is occupied
    SparseArray {
        maximum: 1,
        groups: vec![SparseArrayGroup {
            count: 1,
            elem_size: serialized.len(),
            group: serialized,
            bitmap,
        }],
    }
}

fn slot_data(slot: &SparseArray) -> Option<&[u8]> {
    if slot.groups.is_empty() {
        return None;
    }
    let group = &slot.groups[0];
    if group.count == 0 || group.group.is_empty() {
        return None;
    }
    Some(&group.group[..])
}

fn serialize_bucket(hash: u64, key: &[u8], klen: usize, val: &[u8], vlen: usize) -> Vec<u8> {
    let mut buf: Vec<u8> = Vec::with_capacity(HEADER_BYTES + klen + vlen);
    buf.extend_from_slice(&hash.to_ne_bytes());
    buf.extend_from_slice(&klen.to_ne_bytes());
    buf.extend_from_slice(&vlen.to_ne_bytes());
    let key_copy_len = klen.min(key.len());
    buf.extend_from_slice(&key[..key_copy_len]);
    for _ in key_copy_len..klen {
        buf.push(0);
    }
    let val_copy_len = vlen.min(val.len());
    buf.extend_from_slice(&val[..val_copy_len]);
    for _ in val_copy_len..vlen {
        buf.push(0);
    }
    buf
}

fn deserialize_header(data: &[u8]) -> (u64, usize, usize) {
    let hash = u64::from_ne_bytes(data[0..HASH_BYTES].try_into().unwrap());
    let klen = usize::from_ne_bytes(
        data[HASH_BYTES..HASH_BYTES + USIZE_BYTES].try_into().unwrap(),
    );
    let vlen = usize::from_ne_bytes(
        data[HASH_BYTES + USIZE_BYTES..HEADER_BYTES].try_into().unwrap(),
    );
    (hash, klen, vlen)
}

fn bucket_key_bytes(data: &[u8], klen: usize) -> &[u8] {
    &data[HEADER_BYTES..HEADER_BYTES + klen]
}

fn bucket_val_bytes(data: &[u8], klen: usize, vlen: usize) -> &[u8] {
    let start = HEADER_BYTES + klen;
    &data[start..start + vlen]
}

fn quadratic_probe(key_hash: u64, num_probes: u64, maximum: u64) -> usize {
    // Equivalent to: (key_hash + num_probes*num_probes) & (maximum - 1)
    // when maximum is a power of 2. We use wrapping math to mirror the
    // C version which uses unsigned overflow.
    let h = key_hash.wrapping_add(num_probes.wrapping_mul(num_probes));
    (h & (maximum - 1)) as usize
}

fn rehash_and_grow_table(dict: &mut SparseDict) -> i32 {
    let new_bucket_max = dict.bucket_max * 2;
    let mut new_buckets: Vec<SparseArray> = Vec::with_capacity(new_bucket_max);
    for _ in 0..new_bucket_max {
        new_buckets.push(empty_slot());
    }

    let mut buckets_rehashed: usize = 0;
    let mut old_buckets = std::mem::take(&mut dict.buckets);
    let bucket_count = dict.bucket_count;

    for i in 0..old_buckets.len() {
        let slot = std::mem::replace(&mut old_buckets[i], empty_slot());
        let has_data = !slot.groups.is_empty()
            && slot.groups[0].count > 0
            && !slot.groups[0].group.is_empty();
        if !has_data {
            continue;
        }

        let hash = {
            let data = &slot.groups[0].group;
            u64::from_ne_bytes(data[0..HASH_BYTES].try_into().unwrap())
        };

        let mut num_probes: u64 = 0;
        let probed_val = loop {
            let probed = quadratic_probe(hash, num_probes, new_bucket_max as u64);
            if slot_data(&new_buckets[probed]).is_none() {
                break probed;
            }
            num_probes += 1;
            if num_probes as usize > bucket_count {
                // Restoration on failure: stick the slots back. (Best effort.)
                dict.buckets = old_buckets;
                return 0;
            }
        };

        new_buckets[probed_val] = slot;
        buckets_rehashed += 1;
        if buckets_rehashed == bucket_count {
            break;
        }
    }

    dict.buckets = new_buckets;
    dict.bucket_max = new_bucket_max;
    1
}

// ---------- Sparse Dictionary API ----------

/// Creates a new sparse dictionary.
pub fn sparse_dict_init() -> Option<Box<SparseDict>> {
    let mut buckets: Vec<SparseArray> = Vec::with_capacity(STARTING_SIZE);
    for _ in 0..STARTING_SIZE {
        buckets.push(empty_slot());
    }
    Some(Box::new(SparseDict {
        bucket_max: STARTING_SIZE,
        bucket_count: 0,
        buckets,
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
    let key_bytes = key.as_bytes();
    let key_hash = hash_fnv1a(key_bytes, klen);
    let mut num_probes: u64 = 0;

    loop {
        let probed_val = quadratic_probe(key_hash, num_probes, dict.bucket_max as u64);
        let existing = slot_data(&dict.buckets[probed_val]);

        match existing {
            None => {
                // Empty slot: insert as new.
                let serialized = serialize_bucket(key_hash, key_bytes, klen, value, vlen);
                dict.buckets[probed_val] = make_filled_slot(serialized);
                break;
            }
            Some(data) => {
                let (existing_hash, e_klen, _e_vlen) = deserialize_header(data);
                let e_key = bucket_key_bytes(data, e_klen);
                let key_slice = &key_bytes[..klen.min(key_bytes.len())];
                let key_matches = existing_hash == key_hash
                    && e_klen == klen
                    && e_key.len() == key_slice.len()
                    && e_key == key_slice;
                if key_matches {
                    // Overwrite existing bucket; bucket_count unchanged.
                    let serialized = serialize_bucket(key_hash, key_bytes, klen, value, vlen);
                    dict.buckets[probed_val] = make_filled_slot(serialized);
                    return 1;
                }
            }
        }

        num_probes += 1;
        if num_probes as usize > dict.bucket_count {
            // Could not find an open slot; the table is full / something went wrong.
            return 0;
        }
    }

    dict.bucket_count += 1;

    // Decide whether to resize. Use integer math equivalent to:
    // bucket_count / bucket_max >= RESIZE_PERCENT/100
    if dict.bucket_count * 100 >= dict.bucket_max * RESIZE_PERCENT {
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
    let key_hash = hash_fnv1a(key_bytes, klen);
    let mut num_probes: u64 = 0;

    let mut outsize_opt = outsize;
    loop {
        let probed_val = quadratic_probe(key_hash, num_probes, dict.bucket_max as u64);
        let slot = &dict.buckets[probed_val];
        match slot_data(slot) {
            Some(data) => {
                let (existing_hash, e_klen, e_vlen) = deserialize_header(data);
                let key_slice = &key_bytes[..klen.min(key_bytes.len())];
                if existing_hash == key_hash && e_klen == klen {
                    let e_key = bucket_key_bytes(data, e_klen);
                    if e_key == key_slice {
                        if let Some(out) = outsize_opt.as_deref_mut() {
                            *out = e_vlen;
                        }
                        return Some(bucket_val_bytes(data, e_klen, e_vlen));
                    }
                }
            }
            None => {
                return None;
            }
        }

        num_probes += 1;
        if num_probes as usize > dict.bucket_count {
            return None;
        }
    }
}

/// Frees the sparse dictionary.
pub fn sparse_dict_free(_dict: Box<SparseDict>) -> i32 {
    // Box drops automatically; all owned data is released.
    1
}
