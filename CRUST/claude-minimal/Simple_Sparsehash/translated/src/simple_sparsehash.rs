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

/// Number of bytes used to store the size prefix in front of each value.
const SIZE_BYTES: usize = std::mem::size_of::<usize>();

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

#[inline]
fn full_elem_size(elem_size: usize) -> usize {
    elem_size + SIZE_BYTES
}

#[inline]
fn max_arr_size(maximum: usize) -> usize {
    (maximum - 1) / GROUP_SIZE + 1
}

/// FNV-1a hash, matching the C implementation.
fn hash_fnv1a(key: &str, klen: usize) -> u64 {
    const FNV_PRIME: u64 = 1099511628211;
    const FNV_OFFSET_BIAS: u64 = 14695981039346656037;
    let bytes = key.as_bytes();
    let mut hash: u64 = FNV_OFFSET_BIAS;
    for i in 0..klen {
        hash ^= bytes[i] as u64;
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

/// Counts the number of 1-bits in `bitmap` from position 0 to `position - 1`.
fn position_to_offset(bitmap: &[u32], position: u32) -> u32 {
    let mut retval: u32 = 0;
    let mut pos = position;
    let mut bitmap_iter: usize = 0;
    let bitchunk = BITCHUNK_SIZE as u32;
    while pos >= bitchunk {
        retval += popcount_32(bitmap[bitmap_iter]);
        bitmap_iter += 1;
        pos -= bitchunk;
    }
    let mask = if pos == 0 { 0u32 } else { (1u32 << pos) - 1 };
    retval + popcount_32(bitmap[bitmap_iter] & mask)
}

#[inline]
fn is_position_occupied(bitmap: &[u32], position: u32) -> bool {
    bitmap[charbit(position)] & modbit(position) != 0
}

#[inline]
fn set_position(bitmap: &mut [u32], position: u32) {
    bitmap[charbit(position)] |= modbit(position);
}

/// Sets a value in a sparse array group at logical index `i`.
fn _sparse_array_group_set(
    group: &mut SparseArrayGroup,
    i: u32,
    val: &[u8],
    vlen: usize,
) -> i32 {
    if vlen > group.elem_size {
        return 0;
    }

    let offset = position_to_offset(&group.bitmap, i) as usize;
    let full = full_elem_size(group.elem_size);

    if !is_position_occupied(&group.bitmap, i) {
        let count = group.count as usize;
        let new_total = (count + 1) * full;
        // Resize the underlying buffer to fit one more slot.
        group.group.resize(new_total, 0);

        // Shift later elements up one slot to make room at `offset`.
        if count > offset {
            let start = offset * full;
            let end = count * full;
            let dest = (offset + 1) * full;
            group.group.copy_within(start..end, dest);
        }

        group.count += 1;
        set_position(&mut group.bitmap, i);
    }

    let dest_start = offset * full;

    // Write size prefix.
    let size_bytes = vlen.to_ne_bytes();
    group.group[dest_start..dest_start + SIZE_BYTES].copy_from_slice(&size_bytes);

    // Write value bytes.
    group.group[dest_start + SIZE_BYTES..dest_start + SIZE_BYTES + vlen]
        .copy_from_slice(&val[..vlen]);

    1
}

/// Returns a slice referring to the value stored at logical index `i`,
/// or `None` if the slot is empty.
fn _sparse_array_group_get<'a>(
    group: &'a SparseArrayGroup,
    i: u32,
    outsize: Option<&mut usize>,
) -> Option<&'a [u8]> {
    if !is_position_occupied(&group.bitmap, i) {
        return None;
    }

    let offset = position_to_offset(&group.bitmap, i) as usize;
    let full = full_elem_size(group.elem_size);
    let start = offset * full;

    let mut size_arr = [0u8; SIZE_BYTES];
    size_arr.copy_from_slice(&group.group[start..start + SIZE_BYTES]);
    let item_size = usize::from_ne_bytes(size_arr);

    if item_size == 0 {
        return None;
    }

    if let Some(out) = outsize {
        *out = item_size;
    }

    Some(&group.group[start + SIZE_BYTES..start + SIZE_BYTES + item_size])
}

// ---------- Sparse Array API ----------

/// Creates a new sparse array.
pub fn sparse_array_init(element_size: usize, maximum: u32) -> Option<Box<SparseArray>> {
    if maximum == 0 {
        return None;
    }
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
    let pos = ((i as usize) % GROUP_SIZE) as u32;
    if group_idx >= arr.groups.len() {
        return 0;
    }
    _sparse_array_group_set(&mut arr.groups[group_idx], pos, val, vlen)
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
    let pos = ((i as usize) % GROUP_SIZE) as u32;
    if group_idx >= arr.groups.len() {
        return None;
    }
    _sparse_array_group_get(&arr.groups[group_idx], pos, outsize)
}

/// Frees the sparse array.
pub fn sparse_array_free(arr: Box<SparseArray>) -> i32 {
    drop(arr);
    1
}

// ---------- Sparse Dictionary helpers ----------

/// Reads the raw `*mut SparseBucket` pointer stored in slot `idx`, if any.
fn read_ptr_from_slot(arr: &SparseArray, idx: u32) -> Option<*mut SparseBucket> {
    let bytes = sparse_array_get(arr, idx, None)?;
    if bytes.len() < SIZE_BYTES {
        return None;
    }
    let mut buf = [0u8; SIZE_BYTES];
    buf.copy_from_slice(&bytes[..SIZE_BYTES]);
    let raw_addr = usize::from_ne_bytes(buf);
    if raw_addr == 0 {
        None
    } else {
        Some(raw_addr as *mut SparseBucket)
    }
}

/// Stores a `Box<SparseBucket>` pointer into slot `idx` of the array.
fn store_bucket_box(arr: &mut SparseArray, idx: u32, bucket: Box<SparseBucket>) -> i32 {
    let raw = Box::into_raw(bucket);
    let raw_addr = raw as usize;
    let bytes = raw_addr.to_ne_bytes();
    let result = sparse_array_set(arr, idx, &bytes, SIZE_BYTES);
    if result == 0 {
        // Insertion failed; reclaim and drop the box to avoid leak.
        unsafe {
            drop(Box::from_raw(raw));
        }
    }
    result
}

/// Computes the quadratic probe value: `(hash + n*n) & (max - 1)`.
#[inline]
fn quadratic_probe(key_hash: u64, num_probes: u64, bucket_max: u64) -> u32 {
    (key_hash.wrapping_add(num_probes.wrapping_mul(num_probes)) & (bucket_max - 1)) as u32
}

// ---------- Sparse Dictionary API ----------

/// Creates a new sparse dictionary.
pub fn sparse_dict_init() -> Option<Box<SparseDict>> {
    let inner = sparse_array_init(SIZE_BYTES, STARTING_SIZE as u32)?;
    Some(Box::new(SparseDict {
        bucket_max: STARTING_SIZE,
        bucket_count: 0,
        buckets: vec![*inner],
    }))
}

/// Rehashes the dictionary into a new array of double size.
fn rehash_and_grow(dict: &mut SparseDict) -> i32 {
    let new_bucket_max = dict.bucket_max * 2;
    let mut new_arr = match sparse_array_init(SIZE_BYTES, new_bucket_max as u32) {
        Some(b) => *b,
        None => return 0,
    };

    let old_bucket_max = dict.bucket_max;
    let old_bucket_count = dict.bucket_count;
    let mut buckets_rehashed: usize = 0;

    for i in 0..old_bucket_max {
        // Take ownership of the bucket from the old array (if any).
        let raw_opt = read_ptr_from_slot(&dict.buckets[0], i as u32);
        let bucket_box: Option<Box<SparseBucket>> =
            raw_opt.map(|raw| unsafe { Box::from_raw(raw) });

        if let Some(bucket) = bucket_box {
            let key_hash = bucket.hash;
            let mut num_probes: u64 = 0;

            // Find an empty slot in the new array via quadratic probing.
            let probed_val = loop {
                let pv = quadratic_probe(key_hash, num_probes, new_bucket_max as u64);
                if read_ptr_from_slot(&new_arr, pv).is_none() {
                    break pv;
                }
                if num_probes > old_bucket_count as u64 {
                    // Re-leak the bucket pointer so we don't double-free on failure.
                    let _ = Box::into_raw(bucket);
                    return 0;
                }
                num_probes += 1;
            };

            if store_bucket_box(&mut new_arr, probed_val, bucket) == 0 {
                return 0;
            }
            buckets_rehashed += 1;
            if buckets_rehashed == old_bucket_count {
                break;
            }
        }
    }

    // Replace the old array with the new one. Old array's leftover pointer
    // bytes are just bytes; ownership has already been transferred.
    dict.buckets[0] = new_arr;
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
    let key_hash = hash_fnv1a(key, klen);
    let mut num_probes: u64 = 0;
    let bucket_max_u64 = dict.bucket_max as u64;

    loop {
        let probed_val = quadratic_probe(key_hash, num_probes, bucket_max_u64);

        // Determine slot status without holding a borrow into the array.
        enum Action {
            Empty,
            Replace,
            Probe,
        }
        let action = match read_ptr_from_slot(&dict.buckets[0], probed_val) {
            None => Action::Empty,
            Some(raw) => {
                let existing: &SparseBucket = unsafe { &*raw };
                if existing.hash == key_hash
                    && existing.klen == klen
                    && existing.key.as_bytes() == &key.as_bytes()[..klen]
                {
                    Action::Replace
                } else {
                    Action::Probe
                }
            }
        };

        match action {
            Action::Empty => {
                let bucket = Box::new(SparseBucket {
                    key: key[..klen].to_string(),
                    klen,
                    val: value[..vlen].to_vec(),
                    vlen,
                    hash: key_hash,
                });
                if store_bucket_box(&mut dict.buckets[0], probed_val, bucket) == 0 {
                    return 0;
                }
                break;
            }
            Action::Replace => {
                // Free old bucket and insert the new one in place.
                if let Some(raw) = read_ptr_from_slot(&dict.buckets[0], probed_val) {
                    unsafe {
                        drop(Box::from_raw(raw));
                    }
                }
                let bucket = Box::new(SparseBucket {
                    key: key[..klen].to_string(),
                    klen,
                    val: value[..vlen].to_vec(),
                    vlen,
                    hash: key_hash,
                });
                if store_bucket_box(&mut dict.buckets[0], probed_val, bucket) == 0 {
                    return 0;
                }
                // Replacing does not change bucket_count and skips resize.
                return 1;
            }
            Action::Probe => {
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
        return rehash_and_grow(dict);
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
    let key_hash = hash_fnv1a(key, klen);
    let bucket_max_u64 = dict.bucket_max as u64;
    let mut num_probes: u64 = 0;

    loop {
        let probed_val = quadratic_probe(key_hash, num_probes, bucket_max_u64);
        match read_ptr_from_slot(&dict.buckets[0], probed_val) {
            None => return None,
            Some(raw) => {
                let bucket: &SparseBucket = unsafe { &*raw };
                if bucket.hash == key_hash
                    && bucket.klen == klen
                    && bucket.key.as_bytes() == &key.as_bytes()[..klen]
                {
                    if let Some(out) = outsize {
                        *out = bucket.vlen;
                    }
                    let slice: &[u8] = &bucket.val[..bucket.vlen];
                    // Extend the lifetime to match the dict's borrow. The
                    // bucket lives as long as the dictionary, so this is sound.
                    return Some(unsafe { std::mem::transmute::<&[u8], &'a [u8]>(slice) });
                }
            }
        }

        num_probes += 1;
        if num_probes > dict.bucket_count as u64 {
            return None;
        }
    }
}

/// Frees the sparse dictionary.
pub fn sparse_dict_free(mut dict: Box<SparseDict>) -> i32 {
    let bucket_max = dict.bucket_max;
    // Free every owned bucket pointer stored in the array.
    for i in 0..bucket_max {
        if let Some(raw) = read_ptr_from_slot(&dict.buckets[0], i as u32) {
            unsafe {
                drop(Box::from_raw(raw));
            }
        }
    }
    // Drop the dictionary itself.
    drop(dict.buckets.drain(..));
    drop(dict);
    1
}
