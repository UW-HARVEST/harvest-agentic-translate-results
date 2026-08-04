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

// --- Internal helpers ---

fn hash_fnv1a(key: &[u8]) -> u64 {
    const FNV_PRIME: u64 = 1099511628211;
    const FNV_OFFSET_BIAS: u64 = 14695981039346656037;
    let mut hash = FNV_OFFSET_BIAS;
    // C code uses `uint8_t i` as loop counter, which wraps at 255.
    // iterations = klen, but i is u8, so effectively iterates min(klen, 256) with wrapping.
    // Actually the C code: `const int iterations = klen; uint8_t i; for(i = 0; i < iterations; i++)`
    // When iterations > 255, i wraps around to 0 and keeps going. This is a bug in the C code
    // but we must replicate it for hash compatibility.
    let iterations = key.len() as i32;
    let mut i: u8 = 0;
    loop {
        if (i as i32) >= iterations {
            break;
        }
        hash ^= key[i as usize] as u64;
        hash = hash.wrapping_mul(FNV_PRIME);
        i = i.wrapping_add(1);
    }
    hash
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

fn position_to_offset(bitmap: &[u32; BITMAP_SIZE], position: u32) -> u32 {
    let mut retval = 0u32;
    let mut pos = position;
    let mut bitmap_iter = 0usize;
    while pos >= BITCHUNK_SIZE as u32 {
        retval += popcount_32(bitmap[bitmap_iter]);
        bitmap_iter += 1;
        pos -= BITCHUNK_SIZE as u32;
    }
    retval + popcount_32(bitmap[bitmap_iter] & ((1u32 << pos) - 1))
}

fn is_position_occupied(bitmap: &[u32; BITMAP_SIZE], position: u32) -> bool {
    (bitmap[charbit(position)] & modbit(position)) != 0
}

fn set_position(bitmap: &mut [u32; BITMAP_SIZE], position: u32) {
    bitmap[charbit(position)] |= modbit(position);
}

const USIZE_SIZE: usize = std::mem::size_of::<usize>();

fn sparse_array_group_set(grp: &mut SparseArrayGroup, i: u32, val: &[u8], vlen: usize) -> bool {
    if vlen > grp.elem_size {
        return false;
    }
    let full_elem_size = grp.elem_size + USIZE_SIZE;
    let offset = position_to_offset(&grp.bitmap, i) as usize;

    if !is_position_occupied(&grp.bitmap, i) {
        let old_count = grp.count as usize;
        let to_move = (old_count - offset) * full_elem_size;
        // Expand the group vector by one element
        grp.group.resize((old_count + 1) * full_elem_size, 0);
        // Move existing elements after offset up by one slot
        if to_move > 0 {
            let src = offset * full_elem_size;
            grp.group.copy_within(src..src + to_move, (offset + 1) * full_elem_size);
        }
        grp.count += 1;
        set_position(&mut grp.bitmap, i);
    }

    let base = offset * full_elem_size;
    // Write size prefix
    grp.group[base..base + USIZE_SIZE].copy_from_slice(&vlen.to_ne_bytes());
    // Write data (only vlen bytes, rest stays as-is / zero)
    grp.group[base + USIZE_SIZE..base + USIZE_SIZE + vlen].copy_from_slice(&val[..vlen]);
    true
}

fn sparse_array_group_get<'a>(grp: &'a SparseArrayGroup, i: u32, outsize: Option<&mut usize>) -> Option<&'a [u8]> {
    if !is_position_occupied(&grp.bitmap, i) {
        return None;
    }
    let full_elem_size = grp.elem_size + USIZE_SIZE;
    let offset = position_to_offset(&grp.bitmap, i) as usize;
    let base = offset * full_elem_size;

    let stored_size = usize::from_ne_bytes(grp.group[base..base + USIZE_SIZE].try_into().unwrap());
    if stored_size == 0 {
        return None;
    }
    if let Some(out) = outsize {
        *out = stored_size;
    }
    Some(&grp.group[base + USIZE_SIZE..base + USIZE_SIZE + stored_size])
}

/// Creates a new sparse array.
pub fn sparse_array_init(element_size: usize, maximum: u32) -> Option<Box<SparseArray>> {
    let max_arr_size = (maximum as usize - 1) / GROUP_SIZE + 1;
    let groups = (0..max_arr_size)
        .map(|_| SparseArrayGroup {
            count: 0,
            elem_size: element_size,
            group: Vec::new(),
            bitmap: [0u32; BITMAP_SIZE],
        })
        .collect();
    Some(Box::new(SparseArray {
        maximum: maximum as usize,
        groups,
    }))
}

/// Sets the element at index `i` to `val`.
pub fn sparse_array_set(arr: &mut SparseArray, i: u32, val: &[u8], vlen: usize) -> i32 {
    if i as usize > arr.maximum {
        return 0;
    }
    let grp = &mut arr.groups[i as usize / GROUP_SIZE];
    let position = (i as usize % GROUP_SIZE) as u32;
    if sparse_array_group_set(grp, position, val, vlen) { 1 } else { 0 }
}

/// Retrieves the element at index `i`.
pub fn sparse_array_get<'a>(
    arr: &'a SparseArray,
    i: u32,
    outsize: Option<&mut usize>,
) -> Option<&'a [u8]> {
    if i as usize > arr.maximum {
        return None;
    }
    let grp = &arr.groups[i as usize / GROUP_SIZE];
    let position = (i as usize % GROUP_SIZE) as u32;
    sparse_array_group_get(grp, position, outsize)
}

/// Frees the sparse array.
pub fn sparse_array_free(arr: Box<SparseArray>) -> i32 {
    drop(arr);
    1
}

// --- Sparse Dictionary helpers ---

/// Serialize a SparseBucket into bytes for storage in the sparse array.
/// Format: [hash:8][klen:8][vlen:8][key_bytes][val_bytes]
fn bucket_to_bytes(hash: u64, key: &[u8], val: &[u8]) -> Vec<u8> {
    let mut buf = Vec::with_capacity(8 + USIZE_SIZE + USIZE_SIZE + key.len() + val.len());
    buf.extend_from_slice(&hash.to_ne_bytes());
    buf.extend_from_slice(&key.len().to_ne_bytes());
    buf.extend_from_slice(&val.len().to_ne_bytes());
    buf.extend_from_slice(key);
    buf.extend_from_slice(val);
    buf
}

const BUCKET_HEADER: usize = 8 + USIZE_SIZE + USIZE_SIZE; // hash + klen + vlen

fn bucket_hash(data: &[u8]) -> u64 {
    u64::from_ne_bytes(data[0..8].try_into().unwrap())
}

fn bucket_klen(data: &[u8]) -> usize {
    usize::from_ne_bytes(data[8..8 + USIZE_SIZE].try_into().unwrap())
}

fn bucket_vlen(data: &[u8]) -> usize {
    usize::from_ne_bytes(data[8 + USIZE_SIZE..BUCKET_HEADER].try_into().unwrap())
}

fn bucket_key(data: &[u8]) -> &[u8] {
    let klen = bucket_klen(data);
    &data[BUCKET_HEADER..BUCKET_HEADER + klen]
}

fn bucket_val(data: &[u8]) -> &[u8] {
    let klen = bucket_klen(data);
    let vlen = bucket_vlen(data);
    &data[BUCKET_HEADER + klen..BUCKET_HEADER + klen + vlen]
}

fn quadratic_probe(key_hash: u64, num_probes: usize, maximum: usize) -> usize {
    ((key_hash as usize) + num_probes * num_probes) & (maximum - 1)
}

// ---------- Sparse Dictionary API ----------

/// Creates a new sparse dictionary.
pub fn sparse_dict_init() -> Option<Box<SparseDict>> {
    let arr = sparse_array_init(128, STARTING_SIZE as u32)?;
    Some(Box::new(SparseDict {
        bucket_max: STARTING_SIZE,
        bucket_count: 0,
        buckets: vec![*arr],
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
    let key_bytes = &key.as_bytes()[..klen];
    let key_hash = hash_fnv1a(key_bytes);
    let mut num_probes = 0usize;

    loop {
        let probed_val = quadratic_probe(key_hash, num_probes, dict.bucket_max);
        let mut current_siz = 0usize;
        let existing = sparse_array_get(&dict.buckets[0], probed_val as u32, Some(&mut current_siz));

        if current_siz == 0 && existing.is_none() {
            // Empty slot - insert new bucket
            let serialized = bucket_to_bytes(key_hash, key_bytes, &value[..vlen]);
            if sparse_array_set(&mut dict.buckets[0], probed_val as u32, &serialized, serialized.len()) != 0 {
                break;
            } else {
                return 0;
            }
        } else if let Some(data) = existing {
            // Occupied slot - check if same key
            if bucket_hash(data) == key_hash
                && bucket_klen(data) == klen
                && bucket_key(data) == key_bytes
            {
                // Overwrite existing bucket
                let serialized = bucket_to_bytes(key_hash, key_bytes, &value[..vlen]);
                if sparse_array_set(&mut dict.buckets[0], probed_val as u32, &serialized, serialized.len()) != 0 {
                    return 1; // Don't increment bucket_count
                } else {
                    return 0;
                }
            }
        }

        num_probes += 1;
        if num_probes > dict.bucket_count {
            return 0;
        }
    }

    dict.bucket_count += 1;

    if dict.bucket_count as f64 / dict.bucket_max as f64 >= RESIZE_PERCENT as f64 / 100.0 {
        return rehash_and_grow_table(dict);
    }

    1
}

fn rehash_and_grow_table(dict: &mut SparseDict) -> i32 {
    let new_bucket_max = dict.bucket_max * 2;
    let mut new_arr = match sparse_array_init(128, new_bucket_max as u32) {
        Some(a) => *a,
        None => return 0,
    };

    let mut buckets_rehashed = 0usize;
    for i in 0..dict.bucket_max {
        let mut bucket_siz = 0usize;
        let bucket_data = sparse_array_get(&dict.buckets[0], i as u32, Some(&mut bucket_siz));

        if bucket_siz != 0 {
            if let Some(data) = bucket_data {
                let key_hash = bucket_hash(data);
                let mut num_probes = 0usize;
                let probed_val;
                loop {
                    let pv = quadratic_probe(key_hash, num_probes, new_bucket_max);
                    let mut csiz = 0usize;
                    let cv = sparse_array_get(&new_arr, pv as u32, Some(&mut csiz));
                    if csiz == 0 && cv.is_none() {
                        probed_val = pv;
                        break;
                    }
                    if num_probes > dict.bucket_count {
                        return 0;
                    }
                    num_probes += 1;
                }
                // Copy the bucket data
                let data_copy = data.to_vec();
                if sparse_array_set(&mut new_arr, probed_val as u32, &data_copy, data_copy.len()) == 0 {
                    return 0;
                }
                buckets_rehashed += 1;
            }
        }

        if buckets_rehashed == dict.bucket_count {
            break;
        }
    }

    dict.buckets[0] = new_arr;
    dict.bucket_max = new_bucket_max;
    1
}

/// Retrieves the value associated with a key.
pub fn sparse_dict_get<'a>(
    dict: &'a SparseDict,
    key: &str,
    klen: usize,
    outsize: Option<&mut usize>,
) -> Option<&'a [u8]> {
    let key_bytes = &key.as_bytes()[..klen];
    let key_hash = hash_fnv1a(key_bytes);
    let mut num_probes = 0usize;

    loop {
        let probed_val = quadratic_probe(key_hash, num_probes, dict.bucket_max);
        let mut current_siz = 0usize;
        let existing = sparse_array_get(&dict.buckets[0], probed_val as u32, Some(&mut current_siz));

        if current_siz != 0 {
            if let Some(data) = existing {
                if bucket_hash(data) == key_hash
                    && bucket_klen(data) == klen
                    && bucket_key(data) == key_bytes
                {
                    if let Some(out) = outsize {
                        *out = bucket_vlen(data);
                    }
                    return Some(bucket_val(data));
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
pub fn sparse_dict_free(dict: Box<SparseDict>) -> i32 {
    drop(dict);
    1
}
