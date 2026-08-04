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

// --- private helpers ---

const SIZE_HEADER: usize = 8; // bytes for u64 length prefix on each slot

/// Per-slot element size used by the SparseDict's underlying SparseArray.
/// This must be large enough to hold a serialized SparseBucket
/// (klen + vlen + key + val + 24 bytes of overhead).
const DICT_ELEM_SIZE: usize = 256;

#[inline]
fn full_elem_size(elem_size: usize) -> usize {
    elem_size + SIZE_HEADER
}

#[inline]
fn max_arr_size(maximum: usize) -> usize {
    if maximum == 0 {
        1
    } else {
        (maximum - 1) / GROUP_SIZE + 1
    }
}

#[inline]
fn charbit(position: u32) -> u32 {
    position >> 5
}

#[inline]
fn modbit(position: u32) -> u32 {
    1u32 << (position & 31)
}

fn position_to_offset(bitmap: &[u32; BITMAP_SIZE], position: u32) -> u32 {
    let mut retval: u32 = 0;
    let mut pos = position;
    let mut bitmap_iter: usize = 0;
    while (pos as usize) >= BITCHUNK_SIZE {
        retval += bitmap[bitmap_iter].count_ones();
        pos -= BITCHUNK_SIZE as u32;
        bitmap_iter += 1;
    }
    let mask = 1u32.wrapping_shl(pos).wrapping_sub(1);
    retval + (bitmap[bitmap_iter] & mask).count_ones()
}

fn is_position_occupied(bitmap: &[u32; BITMAP_SIZE], position: u32) -> bool {
    bitmap[charbit(position) as usize] & modbit(position) != 0
}

fn set_position(bitmap: &mut [u32; BITMAP_SIZE], position: u32) {
    bitmap[charbit(position) as usize] |= modbit(position);
}

/// FNV-1a hash, matching C implementation.
fn hash_fnv1a(key: &[u8]) -> u64 {
    const FNV_PRIME: u64 = 1099511628211;
    const FNV_OFFSET_BIAS: u64 = 14695981039346656037;
    let mut hash = FNV_OFFSET_BIAS;
    // C uses uint8_t loop counter: bytes beyond index 255 are not hashed.
    let iterations = std::cmp::min(key.len(), 256);
    for i in 0..iterations {
        hash ^= key[i] as u64;
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    hash
}

#[inline]
fn quadratic_probe(key_hash: u64, num_probes: u32, maximum: usize) -> u32 {
    let np_sq = (num_probes as u64).wrapping_mul(num_probes as u64);
    let mask = (maximum as u64).wrapping_sub(1);
    (key_hash.wrapping_add(np_sq) & mask) as u32
}

// --- internal group helpers ---

fn _sparse_array_group_set(
    arr: &mut SparseArrayGroup,
    position: u32,
    val: &[u8],
    vlen: usize,
) -> i32 {
    if vlen > arr.elem_size {
        return 0;
    }
    if val.len() < vlen {
        return 0;
    }
    let fes = full_elem_size(arr.elem_size);
    let offset = position_to_offset(&arr.bitmap, position) as usize;
    let occupied = is_position_occupied(&arr.bitmap, position);

    if !occupied {
        // Grow storage by one slot
        let to_move_siz = (arr.count as usize - offset) * fes;
        let new_size = (arr.count as usize + 1) * fes;
        arr.group.resize(new_size, 0);
        if to_move_siz > 0 {
            let src = offset * fes;
            let dst = (offset + 1) * fes;
            arr.group.copy_within(src..src + to_move_siz, dst);
        }
        arr.count += 1;
        set_position(&mut arr.bitmap, position);
    }

    // Write size header
    let dest_off = offset * fes;
    let size_bytes = (vlen as u64).to_le_bytes();
    arr.group[dest_off..dest_off + SIZE_HEADER].copy_from_slice(&size_bytes);
    // Write value bytes
    arr.group[dest_off + SIZE_HEADER..dest_off + SIZE_HEADER + vlen]
        .copy_from_slice(&val[..vlen]);
    // Zero the rest of the slot to mimic clean storage (optional, but matches behaviour).
    let pad_start = dest_off + SIZE_HEADER + vlen;
    let pad_end = dest_off + fes;
    if pad_end > pad_start {
        for b in &mut arr.group[pad_start..pad_end] {
            *b = 0;
        }
    }
    1
}

fn _sparse_array_group_get<'a>(
    arr: &'a SparseArrayGroup,
    position: u32,
    outsize: Option<&mut usize>,
) -> Option<&'a [u8]> {
    if !is_position_occupied(&arr.bitmap, position) {
        return None;
    }
    let fes = full_elem_size(arr.elem_size);
    let offset = position_to_offset(&arr.bitmap, position) as usize;
    let byte_off = offset * fes;
    if byte_off + SIZE_HEADER > arr.group.len() {
        return None;
    }
    let size_bytes: [u8; 8] = arr.group[byte_off..byte_off + SIZE_HEADER]
        .try_into()
        .ok()?;
    let len = u64::from_le_bytes(size_bytes) as usize;
    if len == 0 {
        return None;
    }
    if let Some(out) = outsize {
        *out = len;
    }
    Some(&arr.group[byte_off + SIZE_HEADER..byte_off + SIZE_HEADER + len])
}

// --- Sparse Array public API ---

/// Creates a new sparse array.
pub fn sparse_array_init(element_size: usize, maximum: u32) -> Option<Box<SparseArray>> {
    let m = max_arr_size(maximum as usize);
    let mut groups: Vec<SparseArrayGroup> = Vec::with_capacity(m);
    for _ in 0..m {
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
    let position = i % (GROUP_SIZE as u32);
    if group_idx >= arr.groups.len() {
        return 0;
    }
    let group = &mut arr.groups[group_idx];
    _sparse_array_group_set(group, position, val, vlen)
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
    let position = i % (GROUP_SIZE as u32);
    if group_idx >= arr.groups.len() {
        return None;
    }
    let group = &arr.groups[group_idx];
    _sparse_array_group_get(group, position, outsize)
}

/// Frees the sparse array.
pub fn sparse_array_free(_arr: Box<SparseArray>) -> i32 {
    // Box is dropped automatically when this function returns.
    1
}

// --- Sparse Dictionary helpers ---

/// Serialize a SparseBucket into bytes:
/// [klen: 8 le][vlen: 8 le][hash: 8 le][key: klen bytes][val: vlen bytes]
fn serialize_bucket(klen: usize, vlen: usize, hash: u64, key: &[u8], val: &[u8]) -> Vec<u8> {
    let mut buf = Vec::with_capacity(24 + klen + vlen);
    buf.extend_from_slice(&(klen as u64).to_le_bytes());
    buf.extend_from_slice(&(vlen as u64).to_le_bytes());
    buf.extend_from_slice(&hash.to_le_bytes());
    buf.extend_from_slice(&key[..klen]);
    buf.extend_from_slice(&val[..vlen]);
    buf
}

fn deserialize_bucket_meta(buf: &[u8]) -> (usize, usize, u64) {
    let klen = u64::from_le_bytes(buf[0..8].try_into().unwrap()) as usize;
    let vlen = u64::from_le_bytes(buf[8..16].try_into().unwrap()) as usize;
    let hash = u64::from_le_bytes(buf[16..24].try_into().unwrap());
    (klen, vlen, hash)
}

// --- Sparse Dictionary public API ---

/// Creates a new sparse dictionary.
pub fn sparse_dict_init() -> Option<Box<SparseDict>> {
    let arr_box = sparse_array_init(DICT_ELEM_SIZE, STARTING_SIZE as u32)?;
    Some(Box::new(SparseDict {
        bucket_max: STARTING_SIZE,
        bucket_count: 0,
        buckets: vec![*arr_box],
    }))
}

fn rehash_and_grow(dict: &mut SparseDict) -> i32 {
    let new_bucket_max = dict.bucket_max * 2;
    let mut new_buckets_box =
        match sparse_array_init(DICT_ELEM_SIZE, new_bucket_max as u32) {
            Some(b) => b,
            None => return 0,
        };
    let new_buckets: &mut SparseArray = &mut *new_buckets_box;

    // Collect all bucket data from old buckets.
    let mut bucket_data: Vec<Vec<u8>> = Vec::with_capacity(dict.bucket_count);
    {
        let arr = &dict.buckets[0];
        for i in 0..dict.bucket_max {
            let mut sz = 0usize;
            if let Some(buf) = sparse_array_get(arr, i as u32, Some(&mut sz)) {
                if sz != 0 {
                    bucket_data.push(buf.to_vec());
                    if bucket_data.len() == dict.bucket_count {
                        break;
                    }
                }
            }
        }
    }

    // Insert each bucket into new_buckets via quadratic probing.
    for buf in &bucket_data {
        let (_, _, hash) = deserialize_bucket_meta(buf);
        let mut num_probes: u32 = 0;
        let probed_val = loop {
            let pv = quadratic_probe(hash, num_probes, new_bucket_max);
            let mut sz = 0usize;
            let cur = sparse_array_get(new_buckets, pv, Some(&mut sz));
            if cur.is_none() {
                break pv;
            }
            num_probes = num_probes.wrapping_add(1);
            if num_probes as usize > dict.bucket_count {
                return 0;
            }
        };
        if sparse_array_set(new_buckets, probed_val, buf, buf.len()) == 0 {
            return 0;
        }
    }

    // Replace the old buckets array with the new one.
    dict.buckets[0] = *new_buckets_box;
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
    if key_bytes.len() < klen {
        return 0;
    }
    if value.len() < vlen {
        return 0;
    }

    let key_hash = hash_fnv1a(&key_bytes[..klen]);
    let mut num_probes: u32 = 0;

    enum ProbeOutcome {
        Empty(u32),
        Overwrite(u32),
    }

    let outcome: ProbeOutcome = loop {
        let probed_val = quadratic_probe(key_hash, num_probes, dict.bucket_max);

        // Read the existing slot (drop borrow before mutating).
        let probe_state: Option<(usize, u64, Vec<u8>)> = {
            let arr = &dict.buckets[0];
            let mut sz = 0usize;
            match sparse_array_get(arr, probed_val, Some(&mut sz)) {
                Some(buf) if sz != 0 => {
                    let (existing_klen, _existing_vlen, existing_hash) =
                        deserialize_bucket_meta(buf);
                    let existing_key = buf[24..24 + existing_klen].to_vec();
                    Some((existing_klen, existing_hash, existing_key))
                }
                _ => None,
            }
        };

        match probe_state {
            None => {
                break ProbeOutcome::Empty(probed_val);
            }
            Some((existing_klen, existing_hash, existing_key)) => {
                if existing_hash == key_hash
                    && existing_klen == klen
                    && existing_key.as_slice() == &key_bytes[..klen]
                {
                    break ProbeOutcome::Overwrite(probed_val);
                }
            }
        }

        num_probes = num_probes.wrapping_add(1);
        if num_probes as usize > dict.bucket_count {
            return 0;
        }
    };

    let serialized = serialize_bucket(klen, vlen, key_hash, key_bytes, value);
    if serialized.len() > DICT_ELEM_SIZE {
        return 0;
    }

    match outcome {
        ProbeOutcome::Empty(probed_val) => {
            let arr = &mut dict.buckets[0];
            if sparse_array_set(arr, probed_val, &serialized, serialized.len()) == 0 {
                return 0;
            }
            dict.bucket_count += 1;
            // Resize check: bucket_count / bucket_max >= 80%
            if (dict.bucket_count as f32) / (dict.bucket_max as f32)
                >= (RESIZE_PERCENT as f32 / 100.0f32)
            {
                return rehash_and_grow(dict);
            }
            1
        }
        ProbeOutcome::Overwrite(probed_val) => {
            let arr = &mut dict.buckets[0];
            if sparse_array_set(arr, probed_val, &serialized, serialized.len()) == 0 {
                return 0;
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
    outsize: Option<&mut usize>,
) -> Option<&'a [u8]> {
    let key_bytes = key.as_bytes();
    if key_bytes.len() < klen {
        return None;
    }
    let key_hash = hash_fnv1a(&key_bytes[..klen]);
    let mut num_probes: u32 = 0;

    loop {
        let probed_val = quadratic_probe(key_hash, num_probes, dict.bucket_max);
        let mut current_value_siz = 0usize;
        let arr = &dict.buckets[0];
        let current_value = sparse_array_get(arr, probed_val, Some(&mut current_value_siz));

        match current_value {
            Some(buf) if current_value_siz != 0 => {
                let (existing_klen, existing_vlen, existing_hash) = deserialize_bucket_meta(buf);
                let existing_key = &buf[24..24 + existing_klen];
                if existing_hash == key_hash
                    && existing_klen == klen
                    && existing_key == &key_bytes[..klen]
                {
                    if let Some(out) = outsize {
                        *out = existing_vlen;
                    }
                    return Some(&buf[24 + existing_klen..24 + existing_klen + existing_vlen]);
                }
                // No match -> continue probing.
            }
            _ => {
                // Empty slot found -> key not in dict.
                return None;
            }
        }

        num_probes = num_probes.wrapping_add(1);
        if num_probes as usize > dict.bucket_count {
            return None;
        }
    }
}

/// Frees the sparse dictionary.
pub fn sparse_dict_free(_dict: Box<SparseDict>) -> i32 {
    // Box drops automatically.
    1
}
